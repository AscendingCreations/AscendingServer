use std::backtrace::Backtrace;

use chrono::Duration;
use mmap_bytey::MByteBuffer;

use crate::{
    containers::{Entity, GlobalKey, Storage, UserAccess, World},
    gametypes::*,
    maps::{DropItem, get_maps_in_range, try_drop_item},
    players::{
        check_inv_partial_space, check_storage_partial_space, give_inv_item, give_storage_item,
        player_give_vals, player_unequip, player_use_item, save_inv_item, save_storage_item,
        set_inv_slot, set_storage_slot, take_inv_itemslot, take_storage_itemslot,
    },
    socket::{send_fltalert, send_message},
    tasks::{DataTaskToken, unload_entity_packet},
};

use super::SocketID;

pub fn handle_useitem(
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

    let slot = data.read::<u16>()?;

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        if !p_data.combat.death_type.is_alive()
            || p_data.is_using_type.inuse()
            || p_data.combat.attacking
            || p_data.combat.stunned
            || p_data.item_timer.itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        p_data.item_timer.itemtimer =
            *storage.gettick.borrow() + Duration::try_milliseconds(250).unwrap_or_default();
    }

    player_use_item(world, storage, entity, slot)
}

pub fn handle_unequip(
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

    let slot = data.read::<u16>()? as usize;

    let socket_id = if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        if !p_data.combat.death_type.is_alive()
            || p_data.is_using_type.inuse()
            || p_data.combat.attacking
            || p_data.combat.stunned
            || p_data.item_timer.itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        if slot >= EQUIPMENT_TYPE_MAX || p_data.equipment.items[slot].val == 0 {
            return Ok(());
        }

        p_data.socket.id
    } else {
        return Ok(());
    };

    if !player_unequip(world, storage, entity, slot)? {
        send_fltalert(
            storage,
            socket_id,
            "Could not unequiped. No inventory space.".into(),
            FtlType::Error,
        )?;
    }
    Ok(())
}

pub fn handle_switchinvslot(
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

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        let (new_amount, new_slot_val, socket_id) = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive()
                || p_data.combat.attacking
                || p_data.combat.stunned
                || p_data.item_timer.itemtimer > *storage.gettick.borrow()
                || p_data.is_using_type.is_trading()
            {
                return Ok(());
            }

            if oldslot >= MAX_INV || newslot >= MAX_INV || p_data.inventory.items[oldslot].val == 0
            {
                return Ok(());
            }

            (
                amount.min(p_data.inventory.items[oldslot].val),
                p_data.inventory.items[newslot].val,
                p_data.socket.id,
            )
        };

        let mut save_item = false;

        if new_slot_val > 0 {
            let check_result = {
                let p_data = p_data.try_lock()?;

                if p_data.inventory.items[newslot].num == p_data.inventory.items[oldslot].num {
                    1
                } else if p_data.inventory.items[oldslot].val == new_amount {
                    2
                } else {
                    0
                }
            };

            match check_result {
                1 => {
                    let mut itemold = { p_data.try_lock()?.inventory.items[oldslot] };

                    let set_inv_result =
                        set_inv_slot(world, storage, entity, &mut itemold, newslot, new_amount)?;

                    let take_amount = new_amount - set_inv_result;

                    if take_amount > 0 {
                        take_inv_itemslot(world, storage, entity, oldslot, take_amount)?;
                    }
                }
                2 => {
                    let mut p_data = p_data.try_lock()?;

                    let itemnew = p_data.inventory.items[newslot];
                    let itemold = p_data.inventory.items[oldslot];

                    p_data.inventory.items[newslot] = itemold;
                    p_data.inventory.items[oldslot] = itemnew;

                    save_item = true;
                }
                _ => {
                    return send_fltalert(
                        storage,
                        socket_id,
                        "Can not swap slots with a different containing items unless you swap everything".to_string(), 
                        FtlType::Item
                    );
                }
            }
        } else {
            let mut p_data = p_data.try_lock()?;

            let itemnew = p_data.inventory.items[newslot];
            let itemold = p_data.inventory.items[oldslot];

            p_data.inventory.items[newslot] = itemold;
            p_data.inventory.items[oldslot] = itemnew;

            save_item = true;
        }

        if save_item {
            save_inv_item(world, storage, entity, newslot)?;
            save_inv_item(world, storage, entity, oldslot)?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_pickup(
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

    let mut remove_id: Vec<(MapPosition, GlobalKey, Position)> = Vec::new();

    let pos = if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        if !p_data.combat.death_type.is_alive()
            || p_data.is_using_type.inuse()
            || p_data.combat.attacking
            || p_data.combat.stunned
            || p_data.map_timer.mapitemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        p_data.map_timer.mapitemtimer =
            *storage.gettick.borrow() + Duration::try_milliseconds(100).unwrap_or_default();

        p_data.movement.pos
    } else {
        return Ok(());
    };

    let mapids = get_maps_in_range(storage, &pos, 1);
    let mut full_message = false;

    for id in mapids {
        if let Some(x) = id.get() {
            let map = match storage.maps.get(&x) {
                Some(map) => map,
                None => continue,
            }
            .borrow_mut();

            // for the map base data when we need it.
            if storage.bases.maps.get(&pos.map).is_none() {
                continue;
            }
            let ids = map.itemids.clone();

            for i in ids {
                if let Some(Entity::MapItem(mi_data)) = world.get_opt_entity(i) {
                    let mut mapitems = { mi_data.try_lock()?.general };

                    if pos.checkdistance(mapitems.pos.map_offset(id.into())) <= 1 {
                        if mapitems.item.num == 0 {
                            let rem =
                                player_give_vals(world, storage, entity, mapitems.item.val as u64)?;

                            mi_data.try_lock()?.general.item.val = rem as u16;

                            if rem == 0 {
                                remove_id.push((x, i, mapitems.pos));
                            }
                        } else {
                            //let amount = mapitems.item.val;

                            let (is_less, amount, start) = check_inv_partial_space(
                                world,
                                storage,
                                entity,
                                &mut mapitems.item,
                            )?;

                            //if passed then we only get partial of the map item.
                            if is_less {
                                give_inv_item(world, storage, entity, &mut mapitems.item)?;

                                let st = match amount {
                                    0 | 1 => "",
                                    _ => "'s",
                                };

                                if amount != start {
                                    send_message(
                                        world,
                                        storage,
                                        entity,
                                        format!(
                                            "You picked up {} {}{}. Your inventory is Full so some items remain.",
                                            amount,
                                            storage.bases.items[mapitems.item.num as usize].name,
                                            st
                                        ),
                                        String::new(),
                                        MessageChannel::Private,
                                        None,
                                    )?;

                                    mi_data.try_lock()?.general.item.val = start - amount;
                                } else {
                                    send_message(
                                        world,
                                        storage,
                                        entity,
                                        format!(
                                            "You picked up {} {}{}.",
                                            amount,
                                            storage.bases.items[mapitems.item.num as usize].name,
                                            st
                                        ),
                                        String::new(),
                                        MessageChannel::Private,
                                        None,
                                    )?;

                                    remove_id.push((x, i, mapitems.pos));
                                }
                            } else {
                                full_message = true;
                            }
                        }
                    }
                }
            }
        }
    }

    if full_message {
        send_message(
            world,
            storage,
            entity,
            "Your inventory is Full!".to_owned(),
            String::new(),
            MessageChannel::Private,
            None,
        )?;
    }

    for (mappos, entity, pos) in remove_id.iter_mut() {
        if let Some(map) = storage.maps.get(mappos) {
            let mut storage_mapitems = storage.map_items.borrow_mut();
            if storage_mapitems.contains_key(pos) {
                storage_mapitems.swap_remove(pos);
            }
            map.borrow_mut().remove_item(*entity);
            DataTaskToken::EntityUnload(*mappos)
                .add_task(storage, unload_entity_packet(*entity)?)?;
        }
    }

    Ok(())
}

pub fn handle_dropitem(
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

    let slot = data.read::<u16>()? as usize;
    let mut amount = data.read::<u16>()?;

    let (pos, item_data, user_access) =
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive()
                || p_data.is_using_type.inuse()
                || p_data.combat.attacking
                || p_data.combat.stunned
            {
                return Ok(());
            }

            if slot >= MAX_INV || p_data.inventory.items[slot].val == 0 || amount == 0 {
                return Ok(());
            }

            amount = amount.min(p_data.inventory.items[slot].val);

            //make sure it exists first.
            if !storage.bases.maps.contains_key(&p_data.movement.pos.map) {
                return Err(AscendingError::Unhandled(Box::new(Backtrace::capture())));
            }

            (
                p_data.movement.pos,
                p_data.inventory.items[slot],
                p_data.user_access,
            )
        } else {
            return Ok(());
        };

    if try_drop_item(
        world,
        storage,
        DropItem {
            index: item_data.num,
            amount,
            pos,
        },
        match user_access {
            UserAccess::Admin => None,
            _ => Some(
                *storage.gettick.borrow() + Duration::try_milliseconds(600000).unwrap_or_default(),
            ),
        },
        Some(*storage.gettick.borrow() + Duration::try_milliseconds(5000).unwrap_or_default()),
        Some(entity),
    )? {
        take_inv_itemslot(world, storage, entity, slot, amount)?;
    }

    Ok(())
}

pub fn handle_deleteitem(
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

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let slot = data.read::<u16>()? as usize;

        let val = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive()
                || p_data.is_using_type.inuse()
                || p_data.combat.attacking
                || p_data.combat.stunned
            {
                return Ok(());
            }

            if slot >= MAX_INV || p_data.inventory.items[slot].val == 0 {
                return Ok(());
            }

            p_data.inventory.items[slot].val
        };

        take_inv_itemslot(world, storage, entity, slot, val)?;
    }
    Ok(())
}

pub fn handle_switchstorageslot(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let res = {
            let p_data = p_data.try_lock()?;

            !p_data.combat.death_type.is_alive()
                || !p_data.is_using_type.is_bank()
                || p_data.combat.attacking
                || p_data.combat.stunned
        };
        if res {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        let (mut old_slot, new_slot) = {
            let p_data = p_data.try_lock()?;
            (
                p_data.inventory.items[oldslot],
                p_data.inventory.items[newslot],
            )
        };

        if newslot >= MAX_STORAGE || old_slot.val == 0 {
            return Ok(());
        }

        if new_slot.val > 0 {
            if new_slot.num == old_slot.num {
                amount = amount.min(old_slot.val);

                let take_amount = amount
                    - set_storage_slot(world, storage, entity, &mut old_slot, newslot, amount)?;

                if take_amount > 0 {
                    take_storage_itemslot(world, storage, entity, oldslot, take_amount)?;
                }
            } else if old_slot.val == amount {
                {
                    let mut p_data = p_data.try_lock()?;
                    let itemnew = p_data.storage.items[newslot];
                    let itemold = p_data.storage.items[oldslot];

                    p_data.storage.items[newslot] = itemold;
                    p_data.storage.items[oldslot] = itemnew;
                }
                save_storage_item(world, storage, entity, newslot)?;
                save_storage_item(world, storage, entity, oldslot)?;
            } else {
                return send_fltalert(
                    storage,
                    socket_id.id.0,
                    "Can not swap slots with a different containing items unless you swap everything."
                        .into(),
                    FtlType::Error,
                );
            }
        } else {
            {
                let mut p_data = p_data.try_lock()?;
                let itemnew = p_data.storage.items[newslot];
                let itemold = p_data.storage.items[oldslot];

                p_data.storage.items[newslot] = itemold;
                p_data.storage.items[oldslot] = itemnew;
            }
            save_storage_item(world, storage, entity, newslot)?;
            save_storage_item(world, storage, entity, oldslot)?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_deletestorageitem(
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

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let slot = data.read::<u16>()? as usize;

        let val = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive()
                || !p_data.is_using_type.is_bank()
                || p_data.combat.attacking
                || p_data.combat.stunned
            {
                return Ok(());
            }

            if slot >= MAX_STORAGE || p_data.storage.items[slot].val == 0 {
                return Ok(());
            }

            p_data.storage.items[slot].val
        };

        take_storage_itemslot(world, storage, entity, slot, val)?;
    }
    Ok(())
}

pub fn handle_deposititem(
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

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let inv_slot = data.read::<u16>()? as usize;
        let bank_slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        let (mut item_data, storage_data) = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive()
                || !p_data.is_using_type.is_bank()
                || p_data.combat.attacking
                || p_data.combat.stunned
            {
                return Ok(());
            }

            if bank_slot >= MAX_STORAGE
                || p_data.inventory.items[inv_slot].val == 0
                || inv_slot >= MAX_INV
            {
                return Ok(());
            }

            let mut item_data = p_data.inventory.items[inv_slot];

            if item_data.val > amount {
                item_data.val = amount;
            }

            (item_data, p_data.storage.items[bank_slot])
        };

        if storage_data.val == 0 {
            {
                p_data.try_lock()?.storage.items[bank_slot] = item_data;
            }
            save_storage_item(world, storage, entity, bank_slot)?;
            take_inv_itemslot(world, storage, entity, inv_slot, amount)?;
        } else {
            let (is_less, amount, _started) =
                check_storage_partial_space(world, storage, entity, &mut item_data)?;

            if is_less {
                give_storage_item(world, storage, entity, &mut item_data)?;
                take_inv_itemslot(world, storage, entity, inv_slot, amount)?;
            } else {
                send_message(
                    world,
                    storage,
                    entity,
                    "You do not have enough slot to deposit this item!".into(),
                    String::new(),
                    MessageChannel::Private,
                    None,
                )?;
            }
        }
    }

    Ok(())
}

pub fn handle_withdrawitem(
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

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let inv_slot = data.read::<u16>()? as usize;
        let bank_slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        let (mut item_data, inv_data) = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive()
                || !p_data.is_using_type.is_bank()
                || p_data.combat.attacking
                || p_data.combat.stunned
            {
                return Ok(());
            }

            if bank_slot >= MAX_STORAGE
                || p_data.storage.items[bank_slot].val == 0
                || inv_slot >= MAX_INV
            {
                return Ok(());
            }

            let mut item_data = p_data.storage.items[bank_slot];

            if item_data.val > amount {
                item_data.val = amount;
            }

            (item_data, p_data.inventory.items[inv_slot])
        };

        if inv_data.val == 0 {
            {
                p_data.try_lock()?.inventory.items[inv_slot] = item_data;
            }
            save_inv_item(world, storage, entity, inv_slot)?;
            take_storage_itemslot(world, storage, entity, bank_slot, amount)?;
        } else {
            let (is_less, amount, _started) =
                check_inv_partial_space(world, storage, entity, &mut item_data)?;

            if is_less {
                give_inv_item(world, storage, entity, &mut item_data)?;
                take_storage_itemslot(world, storage, entity, bank_slot, amount)?;
            } else {
                send_message(
                    world,
                    storage,
                    entity,
                    "You do not have enough slot to withdraw this item!".into(),
                    String::new(),
                    MessageChannel::Private,
                    None,
                )?;
            }
        }
    }
    Ok(())
}
