use std::{backtrace::Backtrace, sync::Arc};

use crate::{
    containers::{
        Entity, GlobalKey, IsUsingType, PlayerConnectionTimer, Socket, Storage, TradeRequestEntity,
        TradeStatus, UserAccess, World,
    },
    gametypes::*,
    items::Item,
    maps::*,
    players::*,
    socket::*,
    sql::*,
    tasks::*,
};
use chrono::Duration;
use log::{debug, info};
use mio::Token;
use rand::distr::{Alphanumeric, SampleString};
use regex::Regex;

use super::SocketID;

pub fn handle_ping(
    _world: &mut World,
    storage: &Storage,
    _data: &mut MByteBuffer,
    _entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    send_gameping(storage, socket_id.id)
}

pub fn handle_register(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let email = data.read::<String>()?;
    let sprite_id = data.read::<u8>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;

    if entity.is_some() {
        return Err(AscendingError::InvalidSocket);
    }

    let socket = if let Some(client) = storage.server.borrow().clients.get(&socket_id.id) {
        let brw_client = client.borrow();
        Socket {
            addr: Arc::new(brw_client.addr.to_string()),
            id: socket_id.id.0,
        }
    } else {
        return Err(AscendingError::InvalidSocket);
    };

    if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
        return send_infomsg(
            storage,
            socket.id,
            "Client needs to be updated.".into(),
            1,
            true,
        );
    }

    let email_regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )?;

    if !username.chars().all(is_name_acceptable) || !password.chars().all(is_password_acceptable) {
        return send_infomsg(
            storage,
            socket.id,
            "Username or Password contains unaccepted Characters".into(),
            0,
            true,
        );
    }

    if username.len() >= 64 {
        return send_infomsg(
            storage,
            socket.id,
            "Username has too many Characters, 64 Characters Max".into(),
            0,
            true,
        );
    }

    if password.len() >= 128 {
        return send_infomsg(
            storage,
            socket.id,
            "Password has too many Characters, 128 Characters Max".into(),
            0,
            true,
        );
    }

    if !email_regex.is_match(&email) || sprite_id >= 6 {
        return send_infomsg(
            storage,
            socket.id,
            "Email must be an actual email.".into(),
            0,
            true,
        );
    }

    match check_existance(storage, &username, &email) {
        Ok(i) => match i {
            0 => {}
            1 => {
                return send_infomsg(
                    storage,
                    socket.id,
                    "Username Exists. Please try Another.".into(),
                    0,
                    true,
                );
            }
            2 => {
                return send_infomsg(
                    storage,
                    socket.id,
                    "Email Already Exists. Please Try Another.".into(),
                    0,
                    true,
                );
            }
            _ => return Err(AscendingError::RegisterFail),
        },
        Err(_) => return Err(AscendingError::UserNotFound),
    }

    return match new_player(storage, username.clone(), email, password, &socket) {
        Ok(uid) => {
            let code = Alphanumeric.sample_string(&mut rand::rng(), 32);
            let handshake = Alphanumeric.sample_string(&mut rand::rng(), 32);

            // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
            let entity =
                storage.add_player_data(world, code.clone(), handshake.clone(), socket.clone())?;

            if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
                let mut p_data = p_data.try_lock()?;

                p_data.account.username.clone_from(&username);
                p_data.account.id = uid;
                p_data.sprite.id = sprite_id as u16;
            }

            storage
                .hand_shakes
                .borrow_mut()
                .insert(handshake.clone(), entity);

            storage
                .player_names
                .borrow_mut()
                .insert(username.clone(), entity);
            storage
                .player_code
                .borrow_mut()
                .insert(code.to_owned(), entity);

            let tick = *storage.gettick.borrow();

            storage.player_timeout.borrow_mut().insert(
                entity,
                PlayerConnectionTimer(tick + Duration::try_milliseconds(60000).unwrap_or_default()),
            );

            info!(
                "New Player {} with IP {}, Logging in.",
                &username, &socket.addr
            );

            send_myindex(storage, socket.id, entity)?;
            send_codes(world, storage, entity, code, handshake)
        }
        Err(_) => send_infomsg(
            storage,
            socket.id,
            "There was an Issue Creating the player account. Please Contact Support.".into(),
            0,
            true,
        ),
    };
}

pub fn handle_handshake(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    _entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let handshake = data.read::<String>()?;

    let entity = match storage.hand_shakes.borrow_mut().remove(&handshake) {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut can_join = false;

        {
            let mut p_data = p_data.try_lock()?;

            if p_data.login_handshake.handshake == handshake {
                //world.remove_one::<LoginHandShake>(entity)?;

                let _ = storage.player_timeout.borrow_mut().remove(entity);

                p_data.socket.id = socket_id.id.0;

                if let Some(client) = storage.server.borrow().clients.get(&socket_id.id) {
                    client.borrow_mut().entity = Some(entity);
                }

                can_join = true;
            }
        }

        if can_join {
            return joingame(world, storage, entity);
        }
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_login(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;
    let reconnect_code = data.read::<String>()?;

    if entity.is_some() {
        return Err(AscendingError::InvalidSocket);
    }

    let socket = if let Some(client) = storage.server.borrow().clients.get(&socket_id.id) {
        let brw_client = client.borrow();
        Socket {
            addr: Arc::new(brw_client.addr.to_string()),
            id: socket_id.id.0,
        }
    } else {
        return Err(AscendingError::InvalidSocket);
    };

    if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
        return send_infomsg(
            storage,
            socket.id,
            "Client needs to be updated.".into(),
            1,
            true,
        );
    }

    if username.len() >= 64 || password.len() >= 128 {
        return send_infomsg(
            storage,
            socket.id,
            "Account does not Exist or Password is not Correct.".into(),
            0,
            true,
        );
    }

    let id = match find_player(storage, &username, &password)? {
        Some(id) => id,
        None => {
            return send_infomsg(
                storage,
                socket.id,
                "Account does not Exist or Password is not Correct.".into(),
                1,
                true,
            );
        }
    };

    // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
    let code = Alphanumeric.sample_string(&mut rand::rng(), 32);
    let handshake = Alphanumeric.sample_string(&mut rand::rng(), 32);
    let old_entity = { storage.player_names.borrow().get(&username).copied() };

    if let Some(old_entity) = old_entity {
        if let Some(entity) = entity {
            if old_entity != entity {
                if let Some(Entity::Player(p_data)) = world.get_opt_entity(old_entity) {
                    let (old_code, old_socket) = {
                        let p_data = p_data.try_lock()?;
                        (p_data.relogin_code.clone(), p_data.socket.clone())
                    };

                    // if old code is empty means they did get unloaded just not all the way for some reason.
                    if old_code.code.is_empty() {
                        let _ = storage.player_names.borrow_mut().remove(&username);
                    } else if !reconnect_code.is_empty() && old_code.code.contains(&reconnect_code)
                    {
                        if old_socket.id != socket.id {
                            if let Some(client) = storage
                                .server
                                .borrow()
                                .clients
                                .get(&mio::Token(old_socket.id))
                            {
                                client.borrow_mut().close_socket(world, storage)?;
                            } else {
                                return send_swap_error(world, storage, old_socket.id, socket.id);
                            }
                        } else {
                            return send_swap_error(world, storage, old_socket.id, socket.id);
                        }
                    } else {
                        return send_infomsg(
                            storage,
                            socket.id,
                            "Error Loading User.".into(),
                            1,
                            true,
                        );
                    }
                }
            }
        }
    }

    let entity = storage.add_player_data(world, code.clone(), handshake.clone(), socket.clone())?;

    if let Err(_e) = load_player(storage, world, entity, id) {
        return send_infomsg(storage, socket.id, "Error Loading User.".into(), 1, true);
    }

    let tick = *storage.gettick.borrow();

    storage.player_timeout.borrow_mut().insert(
        entity,
        PlayerConnectionTimer(tick + Duration::try_milliseconds(60000).unwrap_or_default()),
    );

    let name = if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let name = p_data.try_lock()?.account.username.clone();

        info!("Player {} with IP: {}, Logging in.", &name, &socket.addr);

        name
    } else {
        return send_infomsg(storage, socket.id, "Error Loading User.".into(), 1, true);
    };

    return send_login_info(world, storage, entity, code, handshake, socket.id, name);
}

pub fn handle_move(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    let dir = data.read::<u8>()?;
    let data_pos = data.read::<Position>()?;

    let (id, pos) = if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        if !p_data.combat.death_type.is_alive()
            || p_data.is_using_type.inuse()
            || p_data.combat.stunned
        {
            return Ok(());
        }

        (p_data.socket.id, p_data.movement.pos)
    } else {
        return Ok(());
    };

    if storage.bases.maps.get(&data_pos.map).is_none() || dir > 3 {
        return Err(AscendingError::InvalidPacket);
    }

    if data_pos != pos {
        player_warp(world, storage, entity, &pos, false)?;
        return Ok(());
    }

    send_move_ok(storage, id, player_movement(world, storage, entity, dir)?)
}

pub fn handle_dir(
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
        let mut p_data = p_data.try_lock()?;

        if !p_data.combat.death_type.is_alive() || p_data.is_using_type.inuse() {
            return Ok(());
        }

        let dir = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        p_data.movement.dir = dir;

        DataTaskToken::Dir(p_data.movement.pos.map).add_task(storage, dir_packet(entity, dir)?)?;
    }
    Ok(())
}

pub fn handle_attack(
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
        let dir = data.read::<u8>()?;
        let target = data.read::<Option<GlobalKey>>()?;

        {
            let mut p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive()
                || p_data.is_using_type.inuse()
                || p_data.combat.attacking
                || p_data.combat.attack_timer.0 > *storage.gettick.borrow()
            {
                return Ok(());
            }

            if dir > 3 {
                return Err(AscendingError::InvalidPacket);
            }

            if p_data.movement.dir != dir {
                p_data.movement.dir = dir;

                DataTaskToken::Dir(p_data.movement.pos.map)
                    .add_task(storage, dir_packet(entity, dir)?)?;
            };

            p_data.combat.attack_timer.0 =
                *storage.gettick.borrow() + Duration::try_milliseconds(250).unwrap_or_default();
        }

        if let Some(target_entity) = target {
            if world.entities.contains_key(target_entity)
                && !player_combat(world, storage, entity, target_entity)?
            {
                player_interact_object(world, storage, entity)?;
            }
        } else {
            player_interact_object(world, storage, entity)?;
        }
    }
    Ok(())
}

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

pub fn handle_message(
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

    let mut usersocket: Option<Token> = None;

    let channel = data.read::<MessageChannel>()?;
    let msg = data.read::<String>()?;
    let name = data.read::<String>()?;

    let (socket_id, p_name) = if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        (p_data.socket.id, p_data.account.username.clone())
    } else {
        return Ok(());
    };

    if msg.len() >= 256 {
        return send_fltalert(
            storage,
            socket_id,
            "Your message is too long. (256 character limit)".into(),
            FtlType::Error,
        );
    }

    match channel {
        MessageChannel::Private => {
            if name.is_empty() {
                return Ok(());
            }

            if name == p_name {
                return send_fltalert(
                    storage,
                    socket_id,
                    "You cannot send messages to yourself".into(),
                    FtlType::Error,
                );
            }

            usersocket = match storage.player_names.borrow().get(&name) {
                Some(id) => {
                    if let Some(Entity::Player(p_data)) = world.get_opt_entity(*id) {
                        Some(Token(p_data.try_lock()?.socket.id))
                    } else {
                        return Ok(());
                    }
                }
                None => {
                    return send_fltalert(
                        storage,
                        socket_id,
                        "Player is offline or does not exist".into(),
                        FtlType::Error,
                    );
                }
            };
        }
        MessageChannel::Map
        | MessageChannel::Global
        | MessageChannel::Trade
        | MessageChannel::Party
        | MessageChannel::Guild
        | MessageChannel::Help
        | MessageChannel::Quest
        | MessageChannel::Npc => {}
    }

    send_message(world, storage, entity, msg, p_name, channel, usersocket)
}

pub fn handle_command(
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

    let command = data.read::<Command>()?;

    match command {
        Command::KickPlayer => {}
        Command::KickPlayerByName(name) => {
            debug!("Kicking Player {:?}", name);
        }
        Command::WarpTo(pos) => {
            debug!("Warping to {:?}", pos);
            player_warp(world, storage, entity, &pos, false)?;
        }
        Command::SpawnNpc(index, pos) => {
            debug!("Spawning NPC {index} on {:?}", pos);
            if let Some(mapdata) = storage.maps.get(&pos.map) {
                let mut data = mapdata.borrow_mut();
                if let Ok(Some(id)) = storage.add_npc(world, index as u64) {
                    data.add_npc(id);
                    spawn_npc(world, pos, None, id)?;
                }
            }
        }
        Command::Trade => {
            if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
                let (target, pos, trade_requesttimer) = {
                    let p1_data = p1_data.try_lock()?;

                    (
                        p1_data.combat.target.target_entity,
                        p1_data.movement.pos,
                        p1_data.trade_request_entity.requesttimer,
                    )
                };

                if let Some(target_entity) = target
                    && world.entities.contains_key(target_entity)
                    && target_entity != entity
                {
                    if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
                        let (target_pos, death_type) = {
                            let p2_data = p2_data.try_lock()?;

                            (p2_data.movement.pos, p2_data.combat.death_type)
                        };

                        //init_trade(world, storage, entity, &target_entity)?;
                        if trade_requesttimer <= *storage.gettick.borrow()
                            && can_target(pos, target_pos, death_type, 1)
                            && can_trade(world, storage, target_entity)?
                        {
                            send_traderequest(world, storage, entity, target_entity)?;

                            {
                                let mut p1_data = p1_data.try_lock()?;
                                let mut p2_data = p2_data.try_lock()?;

                                p1_data.trade_request_entity.entity = Some(target_entity);
                                p1_data.trade_request_entity.requesttimer =
                                    *storage.gettick.borrow()
                                        + Duration::try_milliseconds(60000).unwrap_or_default();
                                // 1 Minute

                                p2_data.trade_request_entity.entity = Some(entity);
                                p2_data.trade_request_entity.requesttimer =
                                    *storage.gettick.borrow()
                                        + Duration::try_milliseconds(60000).unwrap_or_default();
                                // 1 Minute
                            }

                            send_message(
                                world,
                                storage,
                                entity,
                                "Trade Request Sent".into(),
                                String::new(),
                                MessageChannel::Private,
                                None,
                            )?;
                        } else {
                            send_message(
                                world,
                                storage,
                                entity,
                                "Player is busy".into(),
                                String::new(),
                                MessageChannel::Private,
                                None,
                            )?;
                        }
                    }
                } else {
                    send_message(
                        world,
                        storage,
                        entity,
                        "Could not find player".into(),
                        String::new(),
                        MessageChannel::Private,
                        None,
                    )?;
                }
            }
        }
    }

    Ok(())
}

pub fn handle_settarget(
    world: &mut World,
    _storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let target = data.read::<Option<GlobalKey>>()?;

        let mut p_data = p_data.try_lock()?;

        if let Some(target_entity) = target {
            if !world.entities.contains_key(target_entity) {
                return Ok(());
            }
        }

        p_data.combat.target.target_entity = target;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_closestorage(
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

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        {
            let mut p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive() || !p_data.is_using_type.is_bank() {
                return Ok(());
            }

            p_data.is_using_type = IsUsingType::None;
        }
        send_clearisusingtype(world, storage, entity)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_closeshop(
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

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        {
            let mut p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive() || !p_data.is_using_type.is_instore() {
                return Ok(());
            }

            p_data.is_using_type = IsUsingType::None;
        }
        send_clearisusingtype(world, storage, entity)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_closetrade(
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
        let (target_entity, username) = {
            let p1_data = p1_data.try_lock()?;

            if !p1_data.combat.death_type.is_alive() || !p1_data.is_using_type.is_trading() {
                return Ok(());
            }

            if let IsUsingType::Trading(entity) = p1_data.is_using_type {
                (entity, p1_data.account.username.clone())
            } else {
                return Ok(());
            }
        };

        if target_entity == entity {
            return Ok(());
        }

        if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
            close_trade(world, storage, entity)?;
            close_trade(world, storage, target_entity)?;

            {
                p1_data.try_lock()?.trade_request_entity = TradeRequestEntity::default();
                p2_data.try_lock()?.trade_request_entity = TradeRequestEntity::default();
            }

            return send_message(
                world,
                storage,
                target_entity,
                format!("{} has cancelled the trade", username),
                String::new(),
                MessageChannel::Private,
                None,
            );
        }
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_buyitem(
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
        let (is_using_type, player_money) = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive() || !p_data.is_using_type.is_instore() {
                return Ok(());
            }

            (p_data.is_using_type, p_data.money.vals)
        };

        let shop_index = if let IsUsingType::Store(shop) = is_using_type {
            shop
        } else {
            return Ok(());
        };

        let slot = data.read::<u16>()?;

        let shopdata = storage.bases.shops[shop_index as usize].clone();

        if player_money < shopdata.item[slot as usize].price {
            return send_message(
                world,
                storage,
                entity,
                "You do not have enough money".into(),
                String::new(),
                MessageChannel::Private,
                None,
            );
        }

        let mut item = Item {
            num: shopdata.item[slot as usize].index as u32,
            val: shopdata.item[slot as usize].amount,
            ..Default::default()
        };

        if check_inv_space(world, storage, entity, &mut item)? {
            give_inv_item(world, storage, entity, &mut item)?;
            player_take_vals(world, storage, entity, shopdata.item[slot as usize].price)?;
        } else {
            return send_message(
                world,
                storage,
                entity,
                "You do not have enough space in your inventory".into(),
                String::new(),
                MessageChannel::Private,
                None,
            );
        }

        send_message(
            world,
            storage,
            entity,
            "You have successfully bought an item!".into(),
            String::new(),
            MessageChannel::Private,
            None,
        )?;
    }
    Ok(())
}

pub fn handle_sellitem(
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
        let mut amount = data.read::<u16>()?;

        let (is_using_type, inv_item) = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.death_type.is_alive() || !p_data.is_using_type.is_instore() {
                return Ok(());
            }

            (p_data.is_using_type, p_data.inventory.items[slot])
        };

        let _shop_index = if let IsUsingType::Store(shop) = is_using_type {
            shop
        } else {
            return Ok(());
        };

        if slot >= MAX_INV || inv_item.val == 0 {
            return Ok(());
        }

        if amount > inv_item.val {
            amount = inv_item.val;
        };

        let price = if let Some(itemdata) = storage.bases.items.get(inv_item.num as usize) {
            itemdata.baseprice
        } else {
            0
        };

        let total_price = price * amount as u64;
        take_inv_itemslot(world, storage, entity, slot, amount)?;
        player_give_vals(world, storage, entity, total_price)?;

        send_message(
            world,
            storage,
            entity,
            format!("You have sold an item for {}!", total_price),
            String::new(),
            MessageChannel::Private,
            None,
        )?;
    }
    Ok(())
}

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
