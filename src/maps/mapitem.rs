use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    items::Item,
    socket::*,
    tasks::{map_item_packet, unload_entity_packet, DataTaskToken},
    time_ext::MyInstant,
};
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};

use super::{create_mapitem, MapAttribute};

#[derive(Copy, Clone, PartialEq, Eq, Default, MByteBufferRead, MByteBufferWrite)]
pub struct MapItem {
    pub item: Item,
    pub despawn: Option<MyInstant>,
    pub ownertimer: Option<MyInstant>,
    pub ownerid: Option<Entity>,
    pub pos: Position,
}

impl MapItem {
    #[inline(always)]
    pub fn new(num: u32) -> Self {
        let mut item = MapItem::default();
        item.item.num = num;
        item
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct DropItem {
    pub index: u32,
    pub amount: u16,
    pub pos: Position,
}

pub async fn update_map_items(world: &GameWorld, storage: &GameStore) -> Result<()> {
    let tick = *storage.gettick.read().await;

    let mut to_remove = Vec::new();

    {
        let lock = world.read().await;

        for id in &*storage.map_items.read().await {
            let mapitems = lock.get::<&MapItem>(id.1 .0)?;

            if mapitems.despawn.is_some() && lock.get_or_err::<DespawnTimer>(id.1)?.0 <= tick {
                to_remove.push((*id.1, *id.0))
            }
        }
    }

    for (entity, e_pos) in to_remove.iter_mut() {
        if let Some(map) = storage.maps.get(&e_pos.map) {
            let pos = world.get_or_err::<MapItem>(entity).await?.pos;
            let mut storage_mapitems = storage.map_items.write().await;
            if storage_mapitems.contains_key(&pos) {
                storage_mapitems.swap_remove(&pos);
            }
            map.write().await.remove_item(*entity);
            DataTaskToken::EntityUnload(e_pos.map)
                .add_task(storage, unload_entity_packet(*entity)?)
                .await?;
        }
    }

    Ok(())
}

pub async fn find_drop_pos(
    world: &GameWorld,
    storage: &GameStore,
    drop_item: DropItem,
) -> Result<Vec<(Position, Option<Entity>)>> {
    let mut result = Vec::new();

    let storage_mapitem = storage.map_items.read().await;
    let item_base = match storage.bases.items.get(drop_item.index as usize) {
        Some(data) => data,
        None => return Ok(result),
    };

    let mut got_slot = false;
    if !storage_mapitem.contains_key(&drop_item.pos) {
        let mapdata = storage.maps.get(&drop_item.pos.map);
        if let Some(map_data) = mapdata {
            if !map_data
                .read()
                .await
                .is_blocked_tile(drop_item.pos, WorldEntityType::MapItem)
            {
                result.push((drop_item.pos, None));
                got_slot = true;
            }
        }
    } else {
        'endcheck: for x in drop_item.pos.x - 1..=drop_item.pos.x + 1 {
            for y in drop_item.pos.y - 1..=drop_item.pos.y + 1 {
                let mut check_pos = Position {
                    x,
                    y,
                    ..drop_item.pos
                };
                if check_pos.x < 0 {
                    check_pos.x = 31;
                    check_pos.map.x -= 1;
                }
                if check_pos.x >= 32 {
                    check_pos.x = 0;
                    check_pos.map.x += 1;
                }
                if check_pos.y < 0 {
                    check_pos.y = 31;
                    check_pos.map.y -= 1;
                }
                if check_pos.y >= 32 {
                    check_pos.y = 0;
                    check_pos.map.y += 1;
                }

                if !storage_mapitem.contains_key(&check_pos) {
                    let mapdata = storage.maps.get(&check_pos.map);
                    if let Some(map_data) = mapdata {
                        if !map_data
                            .read()
                            .await
                            .is_blocked_tile(check_pos, WorldEntityType::MapItem)
                        {
                            result.push((check_pos, None));
                            got_slot = true;
                            break 'endcheck;
                        }
                    }
                }
            }
        }
    }

    if !got_slot && item_base.stackable {
        let mut leftover = drop_item.amount;

        'endcheck: for x in drop_item.pos.x - 1..=drop_item.pos.x + 1 {
            for y in drop_item.pos.y - 1..=drop_item.pos.y + 1 {
                let mut check_pos = Position {
                    x,
                    y,
                    ..drop_item.pos
                };
                if check_pos.x < 0 {
                    check_pos.x = 31;
                    check_pos.map.x -= 1;
                }
                if check_pos.x >= 32 {
                    check_pos.x = 0;
                    check_pos.map.x += 1;
                }
                if check_pos.y < 0 {
                    check_pos.y = 31;
                    check_pos.map.y -= 1;
                }
                if check_pos.y >= 32 {
                    check_pos.y = 0;
                    check_pos.map.y += 1;
                }

                if let Some(entity) = storage_mapitem.get(&check_pos) {
                    let mapitem = world.get_or_err::<MapItem>(entity).await?;
                    if mapitem.item.num == drop_item.index
                        && mapitem.item.val < item_base.stacklimit
                    {
                        let remaining_val = item_base.stacklimit - mapitem.item.val;
                        leftover = leftover.saturating_sub(remaining_val);
                        result.push((check_pos, Some(*entity)));

                        if leftover == 0 {
                            break 'endcheck;
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

pub async fn try_drop_item(
    world: &GameWorld,
    storage: &GameStore,
    drop_item: DropItem,
    despawn: Option<MyInstant>,
    ownertimer: Option<MyInstant>,
    ownerid: Option<Entity>,
) -> Result<bool> {
    let item_base = match storage.bases.items.get(drop_item.index as usize) {
        Some(data) => data,
        None => return Ok(false),
    };

    // Find open position
    let set_pos = find_drop_pos(world, storage, drop_item).await?;
    if set_pos.is_empty() {
        return Ok(false);
    }

    let mut leftover = drop_item.amount;
    for found_pos in set_pos.iter() {
        if item_base.stackable
            && let Some(got_entity) = found_pos.1
        {
            let lock = world.write().await;
            let map_item = lock.get::<&mut MapItem>(got_entity.0);

            if let Ok(mut mapitem) = map_item {
                mapitem.item.val = mapitem.item.val.saturating_add(leftover);
                if mapitem.item.val > item_base.stacklimit {
                    leftover = mapitem.item.val - item_base.stacklimit;
                    mapitem.item.val = item_base.stacklimit;
                } else {
                    break;
                }
            }
        } else {
            let mut storage_mapitem = storage.map_items.write().await;
            let mapdata = storage.maps.get(&found_pos.0.map);
            if let Some(map_data) = mapdata {
                let mut map_item = create_mapitem(drop_item.index, drop_item.amount, found_pos.0);
                map_item.despawn = despawn;
                map_item.ownertimer = ownertimer;
                map_item.ownerid = ownerid;
                let mut lock = world.write().await;
                let id = lock.spawn((WorldEntityType::MapItem, map_item));
                let despawntimer = if let Some(timer) = despawn {
                    DespawnTimer(timer)
                } else {
                    DespawnTimer::default()
                };
                lock.insert(id, (EntityType::MapItem(Entity(id)), despawntimer))?;
                map_data.write().await.itemids.insert(Entity(id));
                storage_mapitem.insert(found_pos.0, Entity(id));
                DataTaskToken::ItemLoad(found_pos.0.map)
                    .add_task(
                        storage,
                        map_item_packet(
                            Entity(id),
                            map_item.pos,
                            map_item.item,
                            map_item.ownerid,
                            true,
                        )?,
                    )
                    .await?;
            }
            break;
        }
    }

    Ok(true)
}

pub async fn player_interact_object(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
) -> Result<()> {
    if !world.contains(entity).await {
        return Ok(());
    }

    let pos = world.get_or_err::<Position>(entity).await?;
    let dir = world.get_or_err::<Dir>(entity).await?.0;
    let target_pos = match dir {
        1 => {
            let mut next_pos = pos;
            next_pos.x += 1;
            if next_pos.x >= 32 {
                next_pos.x = 0;
                next_pos.map.x += 1;
            }
            next_pos
        }
        2 => {
            let mut next_pos = pos;
            next_pos.y += 1;
            if next_pos.y >= 32 {
                next_pos.y = 0;
                next_pos.map.y += 1;
            }
            next_pos
        }
        3 => {
            let mut next_pos = pos;
            next_pos.x -= 1;
            if next_pos.x < 0 {
                next_pos.x = 31;
                next_pos.map.x -= 1;
            }
            next_pos
        }
        _ => {
            let mut next_pos = pos;
            next_pos.y -= 1;
            if next_pos.y < 0 {
                next_pos.y = 31;
                next_pos.map.y -= 1;
            }
            next_pos
        }
    };

    if let Some(mapdata) = storage.bases.maps.get(&target_pos.map) {
        match mapdata.attribute[target_pos.as_tile()] {
            MapAttribute::Storage => {
                {
                    let lock = world.write().await;
                    *lock.get::<&mut IsUsingType>(entity.0)? = IsUsingType::Bank;
                }
                send_storage(world, storage, entity, 0..35).await?;
                send_storage(world, storage, entity, 35..MAX_STORAGE).await?;
                send_openstorage(world, storage, entity).await
            }
            MapAttribute::Shop(shop_index) => {
                {
                    let lock = world.write().await;
                    *lock.get::<&mut IsUsingType>(entity.0)? =
                        IsUsingType::Store(shop_index as i64);
                }
                send_openshop(world, storage, entity, shop_index).await
            }
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}
