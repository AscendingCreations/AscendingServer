use mmap_bytey::MByteBuffer;

use crate::{
    containers::{Entity, GlobalKey, IsUsingType, Storage, TradeRequestEntity, TradeStatus, World},
    gametypes::*,
    items::Item,
    players::{
        close_trade, count_inv_item, count_trade_item, give_trade_item, init_trade,
        process_player_trade,
    },
    socket::{send_message, send_tradestatus, send_updatetradeitem, send_updatetrademoney},
};

use super::SocketID;

pub fn handle_addtradeitem(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()? as u64;

        let (target_entity, mut inv_item) = {
            let p1_data = p1_data.try_lock()?;

            if !p1_data.combat.death_type.is_alive()
                || !p1_data.is_using_type.is_trading()
                || p1_data.trade_status != TradeStatus::None
            {
                return Ok(());
            }

            let target_entity = if let IsUsingType::Trading(entity) = p1_data.is_using_type {
                entity
            } else {
                return Ok(());
            };

            if target_entity == entity || world.get_opt_entity(target_entity).is_none() {
                return Ok(());
            }

            if slot >= MAX_INV || p1_data.inventory.items[slot].val == 0 {
                return Ok(());
            }

            let mut inv_item = p1_data.inventory.items[slot];

            let base = &storage.bases.items[inv_item.num as usize];
            if base.stackable && amount > base.stacklimit as u64 {
                amount = base.stacklimit as u64
            }

            // Make sure it does not exceed the amount player have
            let inv_count = count_inv_item(inv_item.num, &p1_data.inventory.items);
            let trade_count = count_trade_item(inv_item.num, &p1_data.trade_item.items);
            if trade_count + amount > inv_count {
                amount = inv_count.saturating_sub(trade_count);
            }
            if amount == 0 {
                return Ok(());
            }
            inv_item.val = amount as u16;

            (target_entity, inv_item)
        };

        // Add the item on trade list
        let trade_slot_list = give_trade_item(world, storage, entity, &mut inv_item)?;

        for slot in trade_slot_list.iter() {
            send_updatetradeitem(world, storage, entity, entity, *slot as u16)?;
            send_updatetradeitem(world, storage, entity, target_entity, *slot as u16)?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_removetradeitem(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u64>()?;

        let target_entity = {
            let mut p1_data = p1_data.try_lock()?;

            if !p1_data.combat.death_type.is_alive()
                || !p1_data.is_using_type.is_trading()
                || p1_data.trade_status != TradeStatus::None
            {
                return Ok(());
            }

            let target_entity = if let IsUsingType::Trading(entity) = p1_data.is_using_type {
                entity
            } else {
                return Ok(());
            };

            if target_entity == entity {
                return Ok(());
            }

            let trade_item = p1_data.trade_item.items[slot];

            if slot >= MAX_TRADE_SLOT || trade_item.val == 0 {
                return Ok(());
            }
            amount = amount.min(trade_item.val as u64);

            p1_data.trade_item.items[slot].val = p1_data.trade_item.items[slot]
                .val
                .saturating_sub(amount as u16);
            if p1_data.trade_item.items[slot].val == 0 {
                p1_data.trade_item.items[slot] = Item::default();
            }

            target_entity
        };

        send_updatetradeitem(world, storage, entity, entity, slot as u16)?;
        send_updatetradeitem(world, storage, entity, target_entity, slot as u16)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_updatetrademoney(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
        let target_entity = {
            let mut p1_data = p1_data.try_lock()?;

            let money = p1_data.money.vals;
            let amount = data.read::<u64>()?.min(money);

            if !p1_data.combat.death_type.is_alive()
                || !p1_data.is_using_type.is_trading()
                || p1_data.trade_status != TradeStatus::None
            {
                return Ok(());
            }

            let target_entity = if let IsUsingType::Trading(entity) = p1_data.is_using_type {
                entity
            } else {
                return Ok(());
            };

            if target_entity == entity {
                return Ok(());
            }

            p1_data.trade_money.vals = amount;
            target_entity
        };

        send_updatetrademoney(world, storage, entity, target_entity)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_submittrade(
    world: &mut World,
    storage: &Storage,
    _data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
        let target_entity = {
            let p1_data = p1_data.try_lock()?;

            if !p1_data.combat.death_type.is_alive() || !p1_data.is_using_type.is_trading() {
                return Ok(());
            }

            let target_entity = if let IsUsingType::Trading(entity) = p1_data.is_using_type {
                entity
            } else {
                return Ok(());
            };

            if target_entity == entity {
                return Ok(());
            }

            target_entity
        };

        if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
            let entity_status = { p1_data.try_lock()?.trade_status };
            let target_status = { p2_data.try_lock()?.trade_status };

            match entity_status {
                TradeStatus::None => {
                    p1_data.try_lock()?.trade_status = TradeStatus::Accepted;
                }
                TradeStatus::Accepted => {
                    if target_status == TradeStatus::Accepted {
                        {
                            p1_data.try_lock()?.trade_status = TradeStatus::Submitted;
                        }
                    } else if target_status == TradeStatus::Submitted {
                        {
                            p1_data.try_lock()?.trade_status = TradeStatus::Submitted;
                        }
                        if !process_player_trade(world, storage, entity, target_entity)? {
                            send_message(
                                    world,
                                    storage,
                                    entity,
                                    "One of you does not have enough inventory slot to proceed with the trade".to_string(), String::new(), MessageChannel::Private, None
                                )?;
                            send_message(
                                    world,
                                    storage,
                                    target_entity,
                                    "One of you does not have enough inventory slot to proceed with the trade".to_string(), String::new(), MessageChannel::Private, None
                                )?;
                        }
                        close_trade(world, storage, entity)?;
                        close_trade(world, storage, target_entity)?;
                        return Ok(());
                    }
                }
                _ => {}
            }

            let entity_status = { p1_data.try_lock()?.trade_status };

            send_tradestatus(world, storage, entity, &entity_status, &target_status)?;
            send_tradestatus(
                world,
                storage,
                target_entity,
                &target_status,
                &entity_status,
            )?;

            return Ok(());
        }
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_accepttrade(
    world: &mut World,
    storage: &Storage,
    _data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    let (target_entity, trade_entity, my_status, their_status) = {
        if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
            let mut p1_data = p1_data.try_lock()?;

            if !p1_data.combat.death_type.is_alive() || p1_data.is_using_type.inuse() {
                return Ok(());
            }

            let target_entity = match p1_data.trade_request_entity.entity {
                Some(entity) => entity,
                None => return Ok(()),
            };
            if target_entity == entity {
                return Ok(());
            }

            p1_data.trade_request_entity = TradeRequestEntity::default();

            if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
                let mut p2_data = p2_data.try_lock()?;

                let trade_entity = match p2_data.trade_request_entity.entity {
                    Some(entity) => entity,
                    None => return Ok(()),
                };
                if trade_entity != entity {
                    return Ok(());
                }

                p1_data.trade_status = TradeStatus::None;
                p2_data.trade_status = TradeStatus::None;
                p1_data.trade_request_entity = TradeRequestEntity::default();
                p2_data.trade_request_entity = TradeRequestEntity::default();

                (
                    target_entity,
                    trade_entity,
                    p1_data.trade_status,
                    p2_data.trade_status,
                )
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    };

    send_tradestatus(world, storage, entity, &my_status, &their_status)?;
    send_tradestatus(world, storage, trade_entity, &their_status, &my_status)?;

    init_trade(world, storage, entity, target_entity)
}

pub fn handle_declinetrade(
    world: &mut World,
    storage: &Storage,
    _data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    let target_entity = if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
        let mut p1_data = p1_data.try_lock()?;

        if !p1_data.combat.death_type.is_alive() || p1_data.is_using_type.inuse() {
            return Ok(());
        }

        let target_entity = match p1_data.trade_request_entity.entity {
            Some(entity) => entity,
            None => return Ok(()),
        };

        if target_entity == entity {
            return Ok(());
        }

        if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
            let mut p2_data = p2_data.try_lock()?;

            p1_data.trade_request_entity = TradeRequestEntity::default();

            let trade_entity = match p2_data.trade_request_entity.entity {
                Some(entity) => entity,
                None => return Ok(()),
            };

            if trade_entity != entity {
                return Ok(());
            }

            p2_data.trade_request_entity = TradeRequestEntity::default();

            target_entity
        } else {
            return Ok(());
        }
    } else {
        return Ok(());
    };

    send_message(
        world,
        storage,
        target_entity,
        "Trade Request has been declined".to_string(),
        String::new(),
        MessageChannel::Private,
        None,
    )?;
    send_message(
        world,
        storage,
        entity,
        "Trade Request has been declined".to_string(),
        String::new(),
        MessageChannel::Private,
        None,
    )
}
