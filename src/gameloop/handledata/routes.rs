use std::backtrace::Backtrace;

use crate::{
    containers::Storage, gametypes::*, items::Item, maps::*, players::*, socket::*, sql::*,
    tasks::*,
};
use chrono::Duration;
use hecs::World;
use log::{debug, info};
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;

pub fn handle_ping(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    send_gameping(world, storage, entity)
}

pub fn handle_register(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let email = data.read::<String>()?;
    let sprite_id = data.read::<u8>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;

    if !storage.player_ids.borrow().contains(entity) {
        let (socket_id, address) = {
            let socket = world.get::<&Socket>(entity.0)?;
            (socket.id, socket.addr.clone())
        };

        if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
            return send_infomsg(
                storage,
                socket_id,
                "Client needs to be updated.".into(),
                1,
                true,
            );
        }

        let email_regex = Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )?;

        if !username.chars().all(is_name_acceptable)
            || !password.chars().all(is_password_acceptable)
        {
            return send_infomsg(
                storage,
                socket_id,
                "Username or Password contains unaccepted Characters".into(),
                0,
                true,
            );
        }

        if username.len() >= 64 {
            return send_infomsg(
                storage,
                socket_id,
                "Username has too many Characters, 64 Characters Max".into(),
                0,
                true,
            );
        }

        if password.len() >= 128 {
            return send_infomsg(
                storage,
                socket_id,
                "Password has too many Characters, 128 Characters Max".into(),
                0,
                true,
            );
        }

        if !email_regex.is_match(&email) || sprite_id >= 6 {
            return send_infomsg(
                storage,
                socket_id,
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
                        socket_id,
                        "Username Exists. Please try Another.".into(),
                        0,
                        true,
                    )
                }
                2 => {
                    return send_infomsg(
                        storage,
                        socket_id,
                        "Email Already Exists. Please Try Another.".into(),
                        0,
                        true,
                    )
                }
                _ => return Err(AscendingError::RegisterFail),
            },
            Err(_) => return Err(AscendingError::UserNotFound),
        }

        let code = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        let handshake = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);

        let tick = *storage.gettick.borrow();
        // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
        storage.add_player_data(world, entity, code.clone(), handshake.clone(), tick)?;

        {
            let (account, sprite) = world.query_one_mut::<(&mut Account, &mut Sprite)>(entity.0)?;

            account.username.clone_from(&username);
            sprite.id = sprite_id as u16;
        }

        storage
            .player_names
            .borrow_mut()
            .insert(username.clone(), *entity);

        info!("New Player {} with IP {}, Logging in.", &username, &address);

        return match new_player(storage, world, entity, username, email, password) {
            Ok(uid) => {
                {
                    world.get::<&mut Account>(entity.0)?.id = uid;
                }
                send_myindex(storage, socket_id, entity)?;
                send_codes(world, storage, entity, code, handshake)
            }
            Err(_) => send_infomsg(
                storage,
                socket_id,
                "There was an Issue Creating the player account. Please Contact Support.".into(),
                0,
                true,
            ),
        };
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_handshake(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let handshake = data.read::<String>()?;

    if world.get::<&LoginHandShake>(entity.0)?.handshake == handshake {
        world.remove_one::<LoginHandShake>(entity.0)?;
        world.remove_one::<ConnectionLoginTimer>(entity.0)?;
        return joingame(world, storage, entity);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_login(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;
    let reconnect_code = data.read::<String>()?;

    let (socket_id, address) = {
        let socket = world.get::<&Socket>(entity.0)?;
        (socket.id, socket.addr.clone())
    };

    if !storage.player_ids.borrow().contains(entity) {
        if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
            return send_infomsg(
                storage,
                socket_id,
                "Client needs to be updated.".into(),
                1,
                true,
            );
        }

        if username.len() >= 64 || password.len() >= 128 {
            return send_infomsg(
                storage,
                socket_id,
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
                    socket_id,
                    "Account does not Exist or Password is not Correct.".into(),
                    1,
                    true,
                )
            }
        };

        // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
        let code = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        let handshake = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        let old_entity = { storage.player_names.borrow().get(&username).copied() };

        if let Some(old_entity) = old_entity {
            if old_entity.0 != entity.0 {
                let old_code = world.cloned_get_or_default::<ReloginCode>(&old_entity);

                // if old code is empty means they did get unloaded just not all the way for some reason.
                if old_code.code.is_empty() {
                    let _ = storage.player_names.borrow_mut().remove(&username);
                } else if !reconnect_code.is_empty() && reconnect_code == old_code.code {
                    if let Ok(socket) = world.cloned_get_or_err::<Socket>(&old_entity) {
                        if socket.id != socket_id {
                            if let Some(client) =
                                storage.server.borrow().clients.get(&mio::Token(socket.id))
                            {
                                client.borrow_mut().close_socket(world, storage)?;
                            } else {
                                return send_swap_error(world, storage, socket.id, socket_id);
                            }
                        } else {
                            return send_swap_error(world, storage, socket.id, socket_id);
                        }
                    }
                } else {
                    return send_infomsg(storage, socket_id, "Error Loading User.".into(), 1, true);
                }
            }
        }

        let tick = *storage.gettick.borrow();
        storage.add_player_data(world, entity, code.clone(), handshake.clone(), tick)?;

        if let Err(_e) = load_player(storage, world, entity, id) {
            return send_infomsg(storage, socket_id, "Error Loading User.".into(), 1, true);
        }

        info!("Player {} with IP: {}, Logging in.", &username, address);
        return send_login_info(world, storage, entity, code, handshake, socket_id, username);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_move(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
            || world.get_or_err::<Stunned>(entity)?.0
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let data_pos = data.read::<Position>()?;

        if storage.bases.maps.get(&data_pos.map).is_none() || dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        let pos = world.get_or_err::<Position>(entity)?;

        if data_pos != pos {
            //println!("Desync! {:?} {:?}", data_pos, pos);
            player_warp(world, storage, entity, &pos, false)?;
            return Ok(());
        }
        let id = world.get::<&Socket>(entity.0)?.id;

        return send_move_ok(storage, id, player_movement(world, storage, entity, dir)?);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_dir(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        {
            world.get::<&mut Dir>(entity.0)?.0 = dir;
        }

        DataTaskToken::Dir(world.get_or_err::<Position>(entity)?.map)
            .add_task(storage, dir_packet(*entity, dir)?)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_attack(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<AttackTimer>(entity)?.0 > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let target = data.read::<Option<Entity>>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        if world.get_or_err::<Dir>(entity)?.0 != dir {
            {
                world.get::<&mut Dir>(entity.0)?.0 = dir;
            }
            DataTaskToken::Dir(world.get_or_err::<Position>(entity)?.map)
                .add_task(storage, dir_packet(*entity, dir)?)?;
        };

        if let Some(target_entity) = target {
            if world.contains(target_entity.0) {
                if !player_combat(world, storage, entity, &target_entity)? {
                    player_interact_object(world, storage, entity)?;
                }
                {
                    world.get::<&mut AttackTimer>(entity.0)?.0 = *storage.gettick.borrow()
                        + Duration::try_milliseconds(250).unwrap_or_default();
                }
            }
        } else {
            player_interact_object(world, storage, entity)?;
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_useitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
            || world.get_or_err::<PlayerItemTimer>(entity)?.itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let slot = data.read::<u16>()?;

        {
            world.get::<&mut PlayerItemTimer>(entity.0)?.itemtimer =
                *storage.gettick.borrow() + Duration::try_milliseconds(250).unwrap_or_default();
        }

        return player_use_item(world, storage, entity, slot);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_unequip(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
            || world.get_or_err::<PlayerItemTimer>(entity)?.itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= EQUIPMENT_TYPE_MAX || world.get::<&Equipment>(entity.0)?.items[slot].val == 0 {
            return Ok(());
        }

        if !player_unequip(world, storage, entity, slot)? {
            send_fltalert(
                storage,
                world.get::<&Socket>(entity.0)?.id,
                "Could not unequiped. No inventory space.".into(),
                FtlType::Error,
            )?;
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_switchinvslot(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
            || world.get_or_err::<PlayerItemTimer>(entity)?.itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if oldslot >= MAX_INV
            || newslot >= MAX_INV
            || world.get::<&Inventory>(entity.0)?.items[oldslot].val == 0
        {
            return Ok(());
        }

        amount = amount.min(world.get::<&Inventory>(entity.0)?.items[oldslot].val);

        let mut itemold = world.get::<&Inventory>(entity.0)?.items[oldslot];

        if world.get::<&Inventory>(entity.0)?.items[newslot].val > 0 {
            if world.get::<&Inventory>(entity.0)?.items[newslot].num
                == world.get::<&Inventory>(entity.0)?.items[oldslot].num
            {
                let take_amount =
                    amount - set_inv_slot(world, storage, entity, &mut itemold, newslot, amount)?;
                if take_amount > 0 {
                    take_inv_itemslot(world, storage, entity, oldslot, take_amount)?;
                }
            } else if world.get::<&Inventory>(entity.0)?.items[oldslot].val == amount {
                let itemnew = world.get::<&Inventory>(entity.0)?.items[newslot];
                {
                    world.get::<&mut Inventory>(entity.0)?.items[newslot] = itemold;
                    world.get::<&mut Inventory>(entity.0)?.items[oldslot] = itemnew;
                }
                save_inv_item(world, storage, entity, newslot)?;
                save_inv_item(world, storage, entity, oldslot)?;
            } else {
                return send_fltalert(
                        storage,
                        world.get::<&Socket>(entity.0)?.id,
                        "Can not swap slots with a different containing items unless you swap everything."
                            .into(),
                        FtlType::Error
                    );
            }
        } else {
            let itemnew = world.get::<&Inventory>(entity.0)?.items[newslot];
            {
                world.get::<&mut Inventory>(entity.0)?.items[newslot] = itemold;
                world.get::<&mut Inventory>(entity.0)?.items[oldslot] = itemnew;
            }
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
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        let mut remove_id: Vec<(MapPosition, Entity)> = Vec::new();

        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
            || world.get_or_err::<PlayerMapTimer>(entity)?.mapitemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let mapids = get_maps_in_range(storage, &world.get_or_err::<Position>(entity)?, 1);
        let mut full_message = false;

        for id in mapids {
            if let Some(x) = id.get() {
                let map = match storage.maps.get(&x) {
                    Some(map) => map,
                    None => continue,
                }
                .borrow_mut();

                // for the map base data when we need it.
                if storage
                    .bases
                    .maps
                    .get(&world.get_or_err::<Position>(entity)?.map)
                    .is_none()
                {
                    continue;
                }
                let ids = map.itemids.clone();

                for i in ids {
                    let mut mapitems = world.get_or_err::<MapItem>(&i)?;
                    if world
                        .get_or_err::<Position>(entity)?
                        .checkdistance(mapitems.pos.map_offset(id.into()))
                        <= 1
                    {
                        if mapitems.item.num == 0 {
                            let rem =
                                player_give_vals(world, storage, entity, mapitems.item.val as u64)?;
                            world.get::<&mut MapItem>(i.0)?.item.val = rem as u16;

                            if rem == 0 {
                                remove_id.push((x, i));
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
                                    format!("You picked up {} {}{}. Your inventory is Full so some items remain.", amount, storage.bases.items[mapitems.item.num as usize].name, st),
                                    String::new(),
                                    MessageChannel::Private,
                                    None,
                                )?;
                                    world.get::<&mut MapItem>(i.0)?.item.val = start - amount;
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

                                    remove_id.push((x, i));
                                }
                            } else {
                                full_message = true;
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

        for (mappos, entity) in remove_id.iter_mut() {
            if let Some(map) = storage.maps.get(mappos) {
                let pos = world.get_or_err::<MapItem>(entity)?.pos;
                let mut storage_mapitems = storage.map_items.borrow_mut();
                if storage_mapitems.contains_key(&pos) {
                    storage_mapitems.swap_remove(&pos);
                }
                map.borrow_mut().remove_item(*entity);
                DataTaskToken::EntityUnload(*mappos)
                    .add_task(storage, unload_entity_packet(*entity)?)?;
            }
        }
        {
            world.get::<&mut PlayerMapTimer>(entity.0)?.mapitemtimer =
                *storage.gettick.borrow() + Duration::try_milliseconds(100).unwrap_or_default();
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_dropitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if slot >= MAX_INV || world.get::<&Inventory>(entity.0)?.items[slot].val == 0 || amount == 0
        {
            return Ok(());
        }

        amount = amount.min(world.get::<&Inventory>(entity.0)?.items[slot].val);

        let item_data = world.get::<&Inventory>(entity.0)?.items[slot];

        //make sure it exists first.
        if !storage
            .bases
            .maps
            .contains_key(&world.get_or_err::<Position>(entity)?.map)
        {
            return Err(AscendingError::Unhandled(Box::new(Backtrace::capture())));
        }

        if try_drop_item(
            world,
            storage,
            DropItem {
                index: item_data.num,
                amount,
                pos: world.get_or_err::<Position>(entity)?,
            },
            match world.get_or_err::<UserAccess>(entity)? {
                UserAccess::Admin => None,
                _ => Some(
                    *storage.gettick.borrow()
                        + Duration::try_milliseconds(600000).unwrap_or_default(),
                ),
            },
            Some(*storage.gettick.borrow() + Duration::try_milliseconds(5000).unwrap_or_default()),
            Some(*entity),
        )? {
            take_inv_itemslot(world, storage, entity, slot, amount)?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_deleteitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= MAX_INV || world.get::<&Inventory>(entity.0)?.items[slot].val == 0 {
            return Ok(());
        }

        let val = world.get::<&Inventory>(entity.0)?.items[slot].val;
        take_inv_itemslot(world, storage, entity, slot, val)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_switchstorageslot(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_bank()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
        {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if oldslot >= MAX_STORAGE
            || newslot >= MAX_STORAGE
            || world.get::<&PlayerStorage>(entity.0)?.items[oldslot].val == 0
        {
            return Ok(());
        }

        amount = amount.min(world.get::<&PlayerStorage>(entity.0)?.items[oldslot].val);

        let mut itemold = world.get::<&PlayerStorage>(entity.0)?.items[oldslot];

        if world.get::<&PlayerStorage>(entity.0)?.items[newslot].val > 0 {
            if world.get::<&PlayerStorage>(entity.0)?.items[newslot].num
                == world.get::<&PlayerStorage>(entity.0)?.items[oldslot].num
            {
                let take_amount = amount
                    - set_storage_slot(world, storage, entity, &mut itemold, newslot, amount)?;
                if take_amount > 0 {
                    take_storage_itemslot(world, storage, entity, oldslot, take_amount)?;
                }
            } else if world.get::<&PlayerStorage>(entity.0)?.items[oldslot].val == amount {
                let itemnew = world.get::<&PlayerStorage>(entity.0)?.items[newslot];
                {
                    world.get::<&mut PlayerStorage>(entity.0)?.items[newslot] = itemold;
                    world.get::<&mut PlayerStorage>(entity.0)?.items[oldslot] = itemnew;
                }
                save_storage_item(world, storage, entity, newslot)?;
                save_storage_item(world, storage, entity, oldslot)?;
            } else {
                return send_fltalert(
                        storage,
                        world.get::<&Socket>(entity.0)?.id,
                        "Can not swap slots with a different containing items unless you swap everything."
                            .into(),
                        FtlType::Error
                    );
            }
        } else {
            let itemnew = world.get::<&PlayerStorage>(entity.0)?.items[newslot];
            {
                world.get::<&mut PlayerStorage>(entity.0)?.items[newslot] = itemold;
                world.get::<&mut PlayerStorage>(entity.0)?.items[oldslot] = itemnew;
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
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_bank()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= MAX_STORAGE || world.get::<&PlayerStorage>(entity.0)?.items[slot].val == 0 {
            return Ok(());
        }

        let val = world.get::<&PlayerStorage>(entity.0)?.items[slot].val;
        take_storage_itemslot(world, storage, entity, slot, val)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_deposititem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_bank()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
        {
            return Ok(());
        }

        let inv_slot = data.read::<u16>()? as usize;
        let bank_slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if bank_slot >= MAX_STORAGE
            || inv_slot >= MAX_INV
            || world.get::<&Inventory>(entity.0)?.items[inv_slot].val == 0
        {
            return Ok(());
        }

        let mut item_data = { world.get::<&Inventory>(entity.0)?.items[inv_slot] };

        if item_data.val > amount {
            item_data.val = amount;
        };

        if { world.get::<&PlayerStorage>(entity.0)?.items[bank_slot].val } == 0 {
            {
                world.get::<&mut PlayerStorage>(entity.0)?.items[bank_slot] = item_data;
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

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_withdrawitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_bank()
            || world.get_or_err::<Attacking>(entity)?.0
            || world.get_or_err::<Stunned>(entity)?.0
        {
            return Ok(());
        }

        let inv_slot = data.read::<u16>()? as usize;
        let bank_slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if bank_slot >= MAX_STORAGE
            || world.get::<&PlayerStorage>(entity.0)?.items[bank_slot].val == 0
            || inv_slot >= MAX_INV
        {
            return Ok(());
        }

        let mut item_data = { world.get::<&PlayerStorage>(entity.0)?.items[bank_slot] };

        if item_data.val > amount {
            item_data.val = amount;
        };

        if { world.get::<&Inventory>(entity.0)?.items[inv_slot].val } == 0 {
            {
                world.get::<&mut Inventory>(entity.0)?.items[inv_slot] = item_data;
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
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_message(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        let mut usersocket: Option<usize> = None;

        if !world.get_or_err::<DeathType>(entity)?.is_alive() {
            return Ok(());
        }

        let channel: MessageChannel = data.read()?;

        let msg = data.read::<String>()?;
        let name = data.read::<String>()?;

        if msg.len() >= 256 {
            return send_fltalert(
                storage,
                world.get::<&Socket>(entity.0)?.id,
                "Your message is too long. (256 character limit)".into(),
                FtlType::Error,
            );
        }

        let head = match channel {
            MessageChannel::Map => {
                format!("[Map] {}:", world.get::<&Account>(entity.0)?.username)
            }
            MessageChannel::Global => {
                format!("[Global] {}:", world.get::<&Account>(entity.0)?.username)
            }
            MessageChannel::Trade => {
                format!("[Trade] {}:", world.get::<&Account>(entity.0)?.username)
            }
            MessageChannel::Party => {
                format!("[Party] {}:", world.get::<&Account>(entity.0)?.username)
            }
            MessageChannel::Private => {
                if name.is_empty() {
                    return Ok(());
                }

                if name == world.get::<&Account>(entity.0)?.username {
                    return send_fltalert(
                        storage,
                        world.get::<&Socket>(entity.0)?.id,
                        "You cannot send messages to yourself".into(),
                        FtlType::Error,
                    );
                }

                usersocket = match storage.player_names.borrow().get(&name) {
                    Some(id) => {
                        if let Ok(socket) = world.get::<&Socket>(id.0) {
                            Some(socket.id)
                        } else {
                            return Ok(());
                        }
                    }
                    None => {
                        return send_fltalert(
                            storage,
                            world.get::<&Socket>(entity.0)?.id,
                            "Player is offline or does not exist".into(),
                            FtlType::Error,
                        );
                    }
                };

                format!("[Private] {}:", world.get::<&Account>(entity.0)?.username)
            }
            MessageChannel::Guild => {
                format!("[Guild] {}:", world.get::<&Account>(entity.0)?.username)
            }
            MessageChannel::Help => {
                format!("[Help] {}:", world.get::<&Account>(entity.0)?.username)
            }
            MessageChannel::Quest => "".into(),
            MessageChannel::Npc => "".into(),
        };

        return send_message(world, storage, entity, msg, head, channel, usersocket);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_command(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(_p) = storage.player_ids.borrow().get(entity) {
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
                let target = world.get_or_err::<PlayerTarget>(entity)?.0;
                if let Some(target_entity) = target
                    && world.contains(target_entity.0)
                {
                    //init_trade(world, storage, entity, &target_entity)?;
                    if world.get_or_err::<TradeRequestEntity>(entity)?.requesttimer
                        <= *storage.gettick.borrow()
                        && can_target(
                            world.get_or_err::<Position>(entity)?,
                            world.get_or_err::<Position>(&target_entity)?,
                            world.get_or_err::<DeathType>(&target_entity)?,
                            1,
                        )
                        && can_trade(world, storage, &target_entity)?
                    {
                        send_traderequest(world, storage, entity, &target_entity)?;
                        {
                            if let Ok(mut traderequest) =
                                world.get::<&mut TradeRequestEntity>(entity.0)
                            {
                                traderequest.entity = Some(target_entity);
                                traderequest.requesttimer = *storage.gettick.borrow()
                                    + Duration::try_milliseconds(60000).unwrap_or_default();
                                // 1 Minute
                            }
                            if let Ok(mut traderequest) =
                                world.get::<&mut TradeRequestEntity>(target_entity.0)
                            {
                                traderequest.entity = Some(*entity);
                                traderequest.requesttimer = *storage.gettick.borrow()
                                    + Duration::try_milliseconds(60000).unwrap_or_default();
                                // 1 Minute
                            }
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

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_settarget(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(_p) = storage.player_ids.borrow().get(entity) {
        let target = data.read::<Option<Entity>>()?;

        if let Some(target_entity) = target {
            if !world.contains(target_entity.0) {
                return Ok(());
            }
        }
        world.get::<&mut PlayerTarget>(entity.0)?.0 = target;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_closestorage(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_bank()
        {
            return Ok(());
        }

        {
            *world.get::<&mut IsUsingType>(entity.0)? = IsUsingType::None;
        }
        send_clearisusingtype(world, storage, entity)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_closeshop(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_instore()
        {
            return Ok(());
        }

        {
            *world.get::<&mut IsUsingType>(entity.0)? = IsUsingType::None;
        }
        send_clearisusingtype(world, storage, entity)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_closetrade(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_trading()
        {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity)? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(target_entity.0) {
            return Ok(());
        }

        close_trade(world, storage, entity)?;
        close_trade(world, storage, &target_entity)?;

        {
            *world.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
            *world.get::<&mut TradeRequestEntity>(target_entity.0)? = TradeRequestEntity::default();
        }

        return send_message(
            world,
            storage,
            &target_entity,
            format!(
                "{} has cancelled the trade",
                world.cloned_get_or_err::<Account>(entity)?.username
            ),
            String::new(),
            MessageChannel::Private,
            None,
        );
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_buyitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_instore()
        {
            return Ok(());
        }
        let shop_index =
            if let IsUsingType::Store(shop) = world.get_or_err::<IsUsingType>(entity)? {
                shop
            } else {
                return Ok(());
            };
        let slot = data.read::<u16>()?;

        let shopdata = storage.bases.shops[shop_index as usize].clone();

        let player_money = world.get_or_err::<Money>(entity)?.vals;
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

        return send_message(
            world,
            storage,
            entity,
            "You have successfully bought an item!".into(),
            String::new(),
            MessageChannel::Private,
            None,
        );
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_sellitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_instore()
        {
            return Ok(());
        }
        let _shop_index =
            if let IsUsingType::Store(shop) = world.get_or_err::<IsUsingType>(entity)? {
                shop
            } else {
                return Ok(());
            };
        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if slot >= MAX_INV || world.get::<&Inventory>(entity.0)?.items[slot].val == 0 {
            return Ok(());
        }

        let inv_item = world.cloned_get_or_err::<Inventory>(entity)?.items[slot];
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

        return send_message(
            world,
            storage,
            entity,
            format!("You have sold an item for {}!", total_price),
            String::new(),
            MessageChannel::Private,
            None,
        );
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_addtradeitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_trading()
        {
            return Ok(());
        }
        if world.get_or_err::<TradeStatus>(entity)? != TradeStatus::None {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity)? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(target_entity.0) {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()? as u64;

        if slot >= MAX_INV || world.get::<&Inventory>(entity.0)?.items[slot].val == 0 {
            return Ok(());
        }

        let mut inv_item = world.cloned_get_or_err::<Inventory>(entity)?.items[slot];

        let base = &storage.bases.items[inv_item.num as usize];
        if base.stackable && amount > base.stacklimit as u64 {
            amount = base.stacklimit as u64
        }

        // Make sure it does not exceed the amount player have
        let inv_count = count_inv_item(
            inv_item.num,
            &world.cloned_get_or_err::<Inventory>(entity)?.items,
        );
        let trade_count = count_trade_item(
            inv_item.num,
            &world.cloned_get_or_err::<TradeItem>(entity)?.items,
        );
        if trade_count + amount > inv_count {
            amount = inv_count.saturating_sub(trade_count);
        }
        if amount == 0 {
            return Ok(());
        }
        inv_item.val = amount as u16;

        // Add the item on trade list
        let trade_slot_list = give_trade_item(world, storage, entity, &mut inv_item)?;

        for slot in trade_slot_list.iter() {
            send_updatetradeitem(world, storage, entity, entity, *slot as u16)?;
            send_updatetradeitem(world, storage, entity, &target_entity, *slot as u16)?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_removetradeitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_trading()
        {
            return Ok(());
        }
        if world.get_or_err::<TradeStatus>(entity)? != TradeStatus::None {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity)? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(target_entity.0) {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u64>()?;

        let trade_item = world.cloned_get_or_err::<TradeItem>(entity)?.items[slot];

        if slot >= MAX_TRADE_SLOT || trade_item.val == 0 {
            return Ok(());
        }
        amount = amount.min(trade_item.val as u64);

        {
            if let Ok(mut tradeitem) = world.get::<&mut TradeItem>(entity.0) {
                tradeitem.items[slot].val = tradeitem.items[slot].val.saturating_sub(amount as u16);
                if tradeitem.items[slot].val == 0 {
                    tradeitem.items[slot] = Item::default();
                }
            }
        }

        send_updatetradeitem(world, storage, entity, entity, slot as u16)?;
        send_updatetradeitem(world, storage, entity, &target_entity, slot as u16)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_updatetrademoney(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_trading()
        {
            return Ok(());
        }
        if world.get_or_err::<TradeStatus>(entity)? != TradeStatus::None {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity)? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(target_entity.0) {
            return Ok(());
        }

        let money = world.get_or_err::<Money>(entity)?.vals;
        let amount = data.read::<u64>()?.min(money);

        {
            world.get::<&mut TradeMoney>(entity.0)?.vals = amount;
        }
        send_updatetrademoney(world, storage, entity, &target_entity)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_submittrade(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity)?.is_trading()
        {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity)? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(target_entity.0) {
            return Ok(());
        }

        let entity_status = world.get_or_err::<TradeStatus>(entity)?;
        let target_status = world.get_or_err::<TradeStatus>(&target_entity)?;

        match entity_status {
            TradeStatus::None => {
                *world.get::<&mut TradeStatus>(entity.0)? = TradeStatus::Accepted;
            }
            TradeStatus::Accepted => {
                if target_status == TradeStatus::Accepted {
                    {
                        *world.get::<&mut TradeStatus>(entity.0)? = TradeStatus::Submitted;
                    }
                } else if target_status == TradeStatus::Submitted {
                    {
                        *world.get::<&mut TradeStatus>(entity.0)? = TradeStatus::Submitted;
                    }
                    if !process_player_trade(world, storage, entity, &target_entity)? {
                        send_message(
                            world,
                            storage,
                            entity,
                            "One of you does not have enough inventory slot to proceed with the trade".into(),
                            String::new(),
                            MessageChannel::Private,
                            None,
                        )?;
                        send_message(
                            world,
                            storage,
                            &target_entity,
                            "One of you does not have enough inventory slot to proceed with the trade".into(),
                            String::new(),
                            MessageChannel::Private,
                            None,
                        )?;
                    }
                    close_trade(world, storage, entity)?;
                    close_trade(world, storage, &target_entity)?;
                    return Ok(());
                }
            }
            _ => {}
        }
        send_tradestatus(
            world,
            storage,
            entity,
            &world.get_or_err::<TradeStatus>(entity)?,
            &target_status,
        )?;
        send_tradestatus(
            world,
            storage,
            &target_entity,
            &target_status,
            &world.get_or_err::<TradeStatus>(entity)?,
        )?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_accepttrade(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
        {
            return Ok(());
        }

        let target_entity = match world.get_or_err::<TradeRequestEntity>(entity)?.entity {
            Some(entity) => entity,
            None => return Ok(()),
        };
        {
            *world.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
        }

        if !world.contains(target_entity.0) {
            return Ok(());
        }

        let trade_entity = match world
            .get_or_err::<TradeRequestEntity>(&target_entity)?
            .entity
        {
            Some(entity) => entity,
            None => return Ok(()),
        };
        if trade_entity != *entity || world.get_or_err::<IsUsingType>(&trade_entity)?.inuse() {
            return Ok(());
        }

        {
            *world.get::<&mut TradeStatus>(entity.0)? = TradeStatus::None;
            *world.get::<&mut TradeStatus>(trade_entity.0)? = TradeStatus::None;
            *world.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
            *world.get::<&mut TradeRequestEntity>(target_entity.0)? = TradeRequestEntity::default();
        }
        send_tradestatus(
            world,
            storage,
            entity,
            &world.get_or_err::<TradeStatus>(entity)?,
            &world.get_or_err::<TradeStatus>(&trade_entity)?,
        )?;
        send_tradestatus(
            world,
            storage,
            &trade_entity,
            &world.get_or_err::<TradeStatus>(&trade_entity)?,
            &world.get_or_err::<TradeStatus>(entity)?,
        )?;

        return init_trade(world, storage, entity, &target_entity);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_declinetrade(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_err::<DeathType>(entity)?.is_alive()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
        {
            return Ok(());
        }

        let target_entity = match world.get_or_err::<TradeRequestEntity>(entity)?.entity {
            Some(entity) => entity,
            None => return Ok(()),
        };
        {
            *world.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
        }

        if world.contains(target_entity.0) {
            let trade_entity = match world
                .get_or_err::<TradeRequestEntity>(&target_entity)?
                .entity
            {
                Some(entity) => entity,
                None => return Ok(()),
            };
            if trade_entity == *entity {
                *world.get::<&mut TradeRequestEntity>(target_entity.0)? =
                    TradeRequestEntity::default();
            }
            send_message(
                world,
                storage,
                &target_entity,
                "Trade Request has been declined".into(),
                String::new(),
                MessageChannel::Private,
                None,
            )?;
        }

        return send_message(
            world,
            storage,
            entity,
            "Trade Request has been declined".into(),
            String::new(),
            MessageChannel::Private,
            None,
        );
    }

    Err(AscendingError::InvalidSocket)
}
