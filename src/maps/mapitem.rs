use super::{MapActor, MapAttribute, MapQuickResponse};
use crate::{
    gametypes::*,
    items::Item,
    network::*,
    //tasks::{map_item_packet, unload_entity_packet, DataTaskToken},
    time_ext::MyInstant,
};
use educe::Educe;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use std::backtrace::Backtrace;
use tokio::sync::mpsc;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, MByteBufferRead, MByteBufferWrite)]
pub struct MapItem {
    pub key: EntityKey,
    pub item: Item,
    pub despawn: Option<MyInstant>,
    pub ownertimer: Option<MyInstant>,
    pub ownerid: Option<OwnerID>,
    pub pos: Position,
}

impl MapItem {
    #[inline(always)]
    pub fn new(num: u32) -> Self {
        let mut item = MapItem::default();
        item.item.num = num;
        item
    }

    pub fn new_with(
        item: Item,
        pos: Position,
        despawn: Option<MyInstant>,
        ownerid: Option<OwnerID>,
        ownertimer: Option<MyInstant>,
    ) -> MapItem {
        MapItem {
            item,
            despawn,
            ownertimer,
            ownerid,
            pos,
            key: EntityKey::default(),
        }
    }
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct SpawnItemData {
    pub index: u32,
    pub amount: u16,
    pub pos: Position,
    pub timer_set: u64,
    // Editable
    #[educe(Default = MyInstant::now())]
    pub timer: MyInstant,
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct DropItem {
    pub player: OwnerID,
    pub index: u32,
    pub amount: u16,
    pub pos: Position,
}

/// Used to lock data until an event with its ID requests it.
/// Helps prevent multiple people from getting the same items
/// or duplicating them across maps.
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub enum MapClaims {
    #[default]
    None,
    ItemDrop {
        item: DropItem,
        requests_sent: usize,
    },
    ItemPickup(MapItem),
    Tile(Position),
}

pub fn find_drop_pos(
    map: &MapActor,
    drop_item: DropItem,
) -> Result<Vec<(Position, Option<(EntityKey, u16)>)>> {
    let mut result = Vec::new();
    let mut process_more = false;

    let item_base = match map.storage.bases.items.get(drop_item.index as usize) {
        Some(data) => data,
        None => return Ok(result),
    };

    // Try out main map first.
    'endcheck: for x in drop_item.pos.x - 1..=drop_item.pos.x + 1 {
        for y in drop_item.pos.y - 1..=drop_item.pos.y + 1 {
            if let Some(check_pos) = Position::new_checked(x, y, drop_item.pos.map) {
                if let Some(grid) = map.move_grids.get(&drop_item.pos.map) {
                    if grid[drop_item.pos.as_tile()].item.is_none()
                        && !map.is_blocked_tile(check_pos, WorldEntityType::MapItem)
                    {
                        result.push((check_pos, None));
                        break 'endcheck;
                    }
                }
            }
        }
    }

    if result.is_empty() {
        'endcheck: for x in drop_item.pos.x - 1..=drop_item.pos.x + 1 {
            for y in drop_item.pos.y - 1..=drop_item.pos.y + 1 {
                let check_pos = Position::new_offset(x, y, drop_item.pos.map);

                if check_pos.map == map.position {
                    continue;
                }

                if let Some(grid) = map.move_grids.get(&drop_item.pos.map) {
                    if grid[drop_item.pos.as_tile()].item.is_none()
                        && !map.is_blocked_tile(check_pos, WorldEntityType::MapItem)
                    {
                        result.push((check_pos, None));
                        break 'endcheck;
                    }
                }
            }
        }
    }

    let mut leftover = drop_item.amount;

    if result.is_empty() && item_base.stackable {
        'endcheck: for x in drop_item.pos.x - 1..=drop_item.pos.x + 1 {
            for y in drop_item.pos.y - 1..=drop_item.pos.y + 1 {
                if let Some(check_pos) = Position::new_checked(x, y, drop_item.pos.map) {
                    if let Some(grid) = map.move_grids.get(&drop_item.pos.map) {
                        if let Some((key, item_id, item_val)) = grid[drop_item.pos.as_tile()].item {
                            if item_id == drop_item.index && item_val < item_base.stacklimit {
                                let remaining_val = item_base.stacklimit - item_val;
                                leftover = leftover.saturating_sub(remaining_val);
                                result.push((check_pos, Some((key, remaining_val))));

                                if leftover == 0 {
                                    break 'endcheck;
                                }
                            }
                        }
                    }
                }
            }
        }

        process_more = leftover > 0;
    }

    if process_more {
        'endcheck: for x in drop_item.pos.x - 1..=drop_item.pos.x + 1 {
            for y in drop_item.pos.y - 1..=drop_item.pos.y + 1 {
                let check_pos = Position::new_offset(x, y, drop_item.pos.map);

                if check_pos.map == map.position {
                    continue;
                }

                if let Some(grid) = map.move_grids.get(&drop_item.pos.map) {
                    if let Some((key, item_id, item_val)) = grid[drop_item.pos.as_tile()].item {
                        if item_id == drop_item.index && item_val < item_base.stacklimit {
                            let remaining_val = item_base.stacklimit - item_val;
                            leftover = leftover.saturating_sub(remaining_val);
                            result.push((check_pos, Some((key, remaining_val))));

                            if leftover == 0 {
                                break 'endcheck;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

pub async fn try_drop_item(
    map: &mut MapActor,
    drop_item: DropItem,
    despawn: Option<MyInstant>,
    ownertimer: Option<MyInstant>,
    ownerid: Option<OwnerID>,
) -> Result<(bool, u16)> {
    let mut sent_requests = false;
    let (stack_limit, stackable) = match map.storage.bases.items.get(drop_item.index as usize) {
        Some(data) => (data.stacklimit, data.stackable),
        None => return Ok((false, 0)),
    };

    // Find open position
    let found_positions = find_drop_pos(map, drop_item)?;
    let (map_tx, mut map_rx) = mpsc::channel::<MapQuickResponse>(9);

    if found_positions.is_empty() {
        return Ok((false, 0));
    }

    let mut leftover = drop_item.amount;
    let mut waited_leftover = 0;

    for (position, details) in found_positions.into_iter() {
        if stackable && let Some((key, fits)) = details {
            if position.map != map.position {
                let amount = fits.min(leftover);

                let drop = DropItem {
                    amount,
                    pos: position,
                    ..drop_item
                };

                waited_leftover += amount;

                if let Some(sender) = map.storage.map_senders.get(&position.map) {
                    if let Err(e) = sender
                        .send(super::MapIncomming::RequestItemDrop {
                            map_id: map.position,
                            item: drop,
                            channel: map_tx.clone(),
                        })
                        .await
                    {
                        return Err(AscendingError::TokioMPSCMapSendError {
                            error: Box::new(e),
                            backtrace: Box::new(Backtrace::capture()),
                        });
                    };

                    sent_requests = true;
                    leftover -= amount;
                }
            } else {
                let (key, num, val) = if let Some(item) = map.items.get_mut(key) {
                    let fits = stack_limit - item.item.val;
                    let amount = fits.min(leftover);
                    item.item.val = item.item.val.saturating_add(amount);
                    leftover -= amount;
                    (item.key, item.item.num, item.item.val)
                } else {
                    return Ok((false, 0));
                };

                map.add_item_to_grid(position, key, num, val);
            }
        } else {
            if position.map != map.position {
                let drop = DropItem {
                    pos: position,
                    ..drop_item
                };

                if let Some(sender) = map.storage.map_senders.get(&position.map) {
                    if let Err(e) = sender
                        .send(super::MapIncomming::RequestItemDrop {
                            map_id: map.position,
                            item: drop,
                            channel: map_tx.clone(),
                        })
                        .await
                    {
                        return Err(AscendingError::TokioMPSCMapSendError {
                            error: Box::new(e),
                            backtrace: Box::new(Backtrace::capture()),
                        });
                    };

                    sent_requests = true;
                }
            } else {
                let map_item = MapItem::new_with(
                    Item {
                        num: drop_item.index,
                        val: drop_item.amount,
                        ..Item::default()
                    },
                    position,
                    despawn,
                    ownerid,
                    ownertimer,
                );

                let key = map.items.insert(map_item);

                let mapitem = map.items.get_mut(key).unwrap();
                mapitem.key = key;

                map.add_item_to_grid(position, key, drop_item.index, drop_item.amount);
                /*DataTaskToken::ItemLoad(found_pos.0.map)
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
                .await?;*/
            }
            break;
        }
    }

    // we must drop this or the loop will last forever;
    drop(map_tx);

    if sent_requests {
        while let Some(data) = map_rx.recv().await {
            match data {
                MapQuickResponse::None => {}
                MapQuickResponse::DropItem {
                    map_id,
                    mut item,
                    drop_amount,
                    claim_id,
                } => {
                    if drop_amount > 0 {
                        waited_leftover -= drop_amount;
                        item.amount = drop_amount;

                        if let Some(sender) = map.storage.map_senders.get(&map_id) {
                            if let Err(e) = sender
                                .send(super::MapIncomming::DropItem {
                                    map_id: map.position,
                                    item,
                                    claim_id,
                                })
                                .await
                            {
                                return Err(AscendingError::TokioMPSCMapSendError {
                                    error: Box::new(e),
                                    backtrace: Box::new(Backtrace::capture()),
                                });
                            };
                        }
                    }
                }
            }
        }
    }

    Ok((true, waited_leftover))
}

pub async fn player_interact_object(map: &MapActor, key: EntityKey) -> Result<()> {
    if let Some(player) = map.players.get(key) {
        let target_pos = {
            let player = player.borrow();
            let mut next_pos = player.position;

            match player.dir {
                1 => {
                    next_pos.x += 1;

                    if next_pos.x >= 32 {
                        next_pos.x = 0;
                        next_pos.map.x += 1;
                    }
                }
                2 => {
                    next_pos.y += 1;

                    if next_pos.y >= 32 {
                        next_pos.y = 0;
                        next_pos.map.y += 1;
                    }
                }
                3 => {
                    next_pos.x -= 1;

                    if next_pos.x < 0 {
                        next_pos.x = 31;
                        next_pos.map.x -= 1;
                    }
                }
                _ => {
                    next_pos.y -= 1;

                    if next_pos.y < 0 {
                        next_pos.y = 31;
                        next_pos.map.y -= 1;
                    }
                }
            }

            next_pos
        };

        if let Some(map) = map.storage.bases.maps.get(&map.position) {
            match map.attribute[target_pos.as_tile()] {
                MapAttribute::Storage => {
                    {
                        player.borrow_mut().is_using = IsUsingType::Bank;
                    }

                    //send_storage(world, storage, entity, 0..35).await?;
                    //send_storage(world, storage, entity, 35..MAX_STORAGE).await?;
                    //send_openstorage(world, storage, entity).await
                }
                MapAttribute::Shop(shop_index) => {
                    {
                        player.borrow_mut().is_using = IsUsingType::Store(shop_index as i64);
                    }

                    //send_openshop(world, storage, entity, shop_index).await
                }
                _ => {}
            }
        }
    }

    Ok(())
}
