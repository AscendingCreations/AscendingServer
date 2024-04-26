use crate::{
    containers::Storage,
    gametypes::*,
    items::Item,
    socket::*,
    tasks::{map_item_packet, unload_entity_packet, DataTaskToken},
    time_ext::MyInstant,
};
use bytey::{ByteBufferRead, ByteBufferWrite};
use hecs::World;

use super::{create_mapitem, MapAttribute};

#[derive(Copy, Clone, PartialEq, Eq, Default, ByteBufferRead, ByteBufferWrite)]
pub struct MapItem {
    pub item: Item,
    #[bytey(skip)]
    pub despawn: Option<MyInstant>,
    #[bytey(skip)]
    pub ownertimer: Option<MyInstant>,
    #[bytey(skip)]
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

pub fn update_map_items(world: &mut World, storage: &Storage) -> Result<()> {
    let tick = *storage.gettick.borrow();

    let mut to_remove = Vec::new();

    for id in &*storage.map_items.borrow() {
        let mapitems = world.get_or_err::<MapItem>(id.1)?;
        if mapitems.despawn.is_some() && world.get_or_err::<DespawnTimer>(id.1)?.0 <= tick {
            to_remove.push((*id.1, *id.0))
        }
    }

    for (entity, e_pos) in to_remove.iter_mut() {
        if let Some(map) = storage.maps.get(&e_pos.map) {
            let pos = world.get_or_err::<MapItem>(entity)?.pos;
            let mut storage_mapitems = storage.map_items.borrow_mut();
            if storage_mapitems.contains_key(&pos) {
                storage_mapitems.swap_remove(&pos);
            }
            map.borrow_mut().remove_item(*entity);
            DataTaskToken::EntityUnload(e_pos.map)
                .add_task(storage, unload_entity_packet(*entity)?)?;
        }
    }

    Ok(())
}

pub fn find_drop_pos(
    world: &mut World,
    storage: &Storage,
    drop_item: DropItem,
) -> Result<Vec<(Position, Option<Entity>)>> {
    let mut result = Vec::new();

    let storage_mapitem = storage.map_items.borrow_mut();
    let item_base = match storage.bases.items.get(drop_item.index as usize) {
        Some(data) => data,
        None => return Ok(result),
    };

    let mut got_slot = false;
    if !storage_mapitem.contains_key(&drop_item.pos) {
        let mapdata = storage.maps.get(&drop_item.pos.map);
        if let Some(map_data) = mapdata {
            if !map_data
                .borrow()
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
                            .borrow()
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
                    let mapitem = world.get_or_err::<MapItem>(entity)?;
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

pub fn try_drop_item(
    world: &mut World,
    storage: &Storage,
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
    let set_pos = find_drop_pos(world, storage, drop_item)?;
    if set_pos.is_empty() {
        return Ok(false);
    }

    let mut leftover = drop_item.amount;
    for found_pos in set_pos.iter() {
        if item_base.stackable
            && let Some(got_entity) = found_pos.1
        {
            if let Ok(mut mapitem) = world.get::<&mut MapItem>(got_entity.0) {
                mapitem.item.val = mapitem.item.val.saturating_add(leftover);
                if mapitem.item.val > item_base.stacklimit {
                    leftover = mapitem.item.val - item_base.stacklimit;
                    mapitem.item.val = item_base.stacklimit;
                } else {
                    break;
                }
            }
        } else {
            let mut storage_mapitem = storage.map_items.borrow_mut();
            let mapdata = storage.maps.get(&found_pos.0.map);
            if let Some(map_data) = mapdata {
                let mut map_item = create_mapitem(drop_item.index, drop_item.amount, found_pos.0);
                map_item.despawn = despawn;
                map_item.ownertimer = ownertimer;
                map_item.ownerid = ownerid;
                let id = world.spawn((WorldEntityType::MapItem, map_item));
                let despawntimer = if let Some(timer) = despawn {
                    DespawnTimer(timer)
                } else {
                    DespawnTimer::default()
                };
                world.insert(id, (EntityType::MapItem(Entity(id)), despawntimer))?;
                map_data.borrow_mut().itemids.insert(Entity(id));
                storage_mapitem.insert(found_pos.0, Entity(id));
                DataTaskToken::ItemLoad(found_pos.0.map).add_task(
                    storage,
                    map_item_packet(
                        Entity(id),
                        map_item.pos,
                        map_item.item,
                        map_item.ownerid,
                        true,
                    )?,
                )?;
            }
            break;
        }
    }

    Ok(true)
}

pub fn player_interact_object(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    if !world.contains(entity.0) {
        return Ok(());
    }

    let pos = world.get_or_err::<Position>(entity)?;
    let dir = world.get_or_err::<Dir>(entity)?.0;
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
                *world.get::<&mut IsUsingType>(entity.0)? = IsUsingType::Bank;
                send_storage(world, storage, entity)?;
                send_openstorage(world, storage, entity)
            }
            MapAttribute::Shop(shop_index) => {
                *world.get::<&mut IsUsingType>(entity.0)? = IsUsingType::Store(shop_index as i64);
                send_openshop(world, storage, entity, shop_index)
            }
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}
