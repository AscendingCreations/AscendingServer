use std::backtrace::Backtrace;

use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    items::Item,
    maps::*,
    players::*,
    network::*,
    sql::*,
    tasks::*,
    WorldExtrasAsync,
};
use chrono::Duration;
use hecs::MissingComponent;
use log::{debug, info};
use rand::distributions::{Alphanumeric, DistString};
use regex::Regex;

pub async fn handle_ping(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    send_gameping(world, storage, entity).await
}

pub async fn handle_register(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let email = data.read::<String>()?;
    let sprite_id = data.read::<u8>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;

    if !storage.player_ids.read().await.contains(entity) {
        let (socket_id, address) = {
            let lock = world.read().await;
            let socket = lock.get::<&Socket>(entity.0)?;
            (socket.id, socket.addr.clone())
        };

        if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
            return send_infomsg(storage, socket_id, "Client needs to be updated.".into(), 1).await;
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
            )
            .await;
        }

        if username.len() >= 64 {
            return send_infomsg(
                storage,
                socket_id,
                "Username has too many Characters, 64 Characters Max".into(),
                0,
            )
            .await;
        }

        if password.len() >= 128 {
            return send_infomsg(
                storage,
                socket_id,
                "Password has too many Characters, 128 Characters Max".into(),
                0,
            )
            .await;
        }

        if !email_regex.is_match(&email) || sprite_id >= 6 {
            return send_infomsg(
                storage,
                socket_id,
                "Email must be an actual email.".into(),
                0,
            )
            .await;
        }

        match check_existance(storage, &username, &email).await {
            Ok(i) => match i {
                0 => {}
                1 => {
                    return send_infomsg(
                        storage,
                        socket_id,
                        "Username Exists. Please try Another.".into(),
                        0,
                    )
                    .await;
                }
                2 => {
                    return send_infomsg(
                        storage,
                        socket_id,
                        "Email Already Exists. Please Try Another.".into(),
                        0,
                    )
                    .await;
                }
                _ => return Err(AscendingError::RegisterFail),
            },
            Err(_) => return Err(AscendingError::UserNotFound),
        }

        let code = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        let handshake = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);

        let tick = *storage.gettick.read().await;
        // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
        storage
            .add_player_data(world, entity, code.clone(), handshake.clone(), tick)
            .await?;

        {
            let lock = world.read().await;
            let mut query = lock.query_one::<(&mut Account, &mut Sprite)>(entity.0)?;
            let (account, sprite) = query.get().ok_or(AscendingError::HecsComponent {
                error: hecs::ComponentError::MissingComponent(MissingComponent::new::<(
                    &mut Account,
                    &mut Sprite,
                )>()),
                backtrace: Box::new(Backtrace::capture()),
            })?;

            account.username.clone_from(&username);
            account.email.clone_from(&email);
            sprite.id = sprite_id as u16;
        }

        storage
            .player_emails
            .write()
            .await
            .insert(email.clone(), *entity);

        storage
            .player_usernames
            .write()
            .await
            .insert(username.clone(), *entity);

        info!("New Player {} with IP {}, Logging in.", &username, &address);

        return match new_player(storage, world, entity, username, email, password).await {
            Ok(uid) => {
                {
                    let lock = world.write().await;
                    lock.get::<&mut Account>(entity.0)?.id = uid;
                }
                send_myindex(storage, socket_id, entity).await?;
                send_codes(world, storage, entity, code, handshake).await
            }
            Err(_) => {
                send_infomsg(
                    storage,
                    socket_id,
                    "There was an Issue Creating the player account. Please Contact Support."
                        .into(),
                    0,
                )
                .await
            }
        };
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_handshake(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let handshake = data.read::<String>()?;
    let user_handshake = world.cloned_get_or_err::<LoginHandShake>(entity).await?;

    if user_handshake.handshake == handshake {
        {
            let mut lock = world.write().await;
            lock.remove_one::<LoginHandShake>(entity.0)?;
            lock.remove_one::<ConnectionLoginTimer>(entity.0)?;
        }
        return joingame(world, storage, entity).await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_login(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let email = data.read::<String>()?;
    let password = data.read::<String>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;
    let reconnect_code = data.read::<String>()?;

    let (socket_id, address) = {
        let lock = world.read().await;
        let socket = lock.get::<&Socket>(entity.0)?;
        (socket.id, socket.addr.clone())
    };

    if !storage.player_ids.read().await.contains(entity) {
        if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
            return send_infomsg(storage, socket_id, "Client needs to be updated.".into(), 1).await;
        }

        if email.len() >= 64 || password.len() >= 128 {
            return send_infomsg(
                storage,
                socket_id,
                "Account does not Exist or Password is not Correct.".into(),
                0,
            )
            .await;
        }

        let id = match find_player(storage, &email, &password).await? {
            Some(id) => id,
            None => {
                return send_infomsg(
                    storage,
                    socket_id,
                    "Account does not Exist or Password is not Correct.".into(),
                    1,
                )
                .await;
            }
        };

        // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
        let code = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        let handshake = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
        let old_entity = {
            let names_lock = storage.player_emails.read().await;
            names_lock.get(&email).copied()
        };

        if let Some(old_entity) = old_entity {
            if old_entity.0 != entity.0 {
                let old_code = world
                    .cloned_get_or_default::<ReloginCode>(&old_entity)
                    .await;

                // if old code is empty means they did get unloaded just not all the way for some reason.
                if old_code.code.is_empty() {
                    let _ = storage.player_emails.write().await.remove(&email);
                } else if !reconnect_code.is_empty() && reconnect_code == old_code.code {
                    if let Ok(socket) = world.cloned_get_or_err::<Socket>(&old_entity).await {
                        if socket.id != socket_id {
                            if let Some(client) = storage
                                .server
                                .read()
                                .await
                                .clients
                                .get(&mio::Token(socket.id))
                            {
                                client.lock().await.close_socket(world, storage).await?;
                            } else {
                                return send_swap_error(world, storage, socket.id, socket_id).await;
                            }
                        } else {
                            return send_swap_error(world, storage, socket.id, socket_id).await;
                        }
                    }
                } else {
                    return send_infomsg(
                        storage,
                        socket_id,
                        "User Already Online, If you think this is an error please report it."
                            .into(),
                        1,
                    )
                    .await;
                }
            }
        }

        let tick = *storage.gettick.read().await;
        storage
            .add_player_data(world, entity, code.clone(), handshake.clone(), tick)
            .await?;

        if let Err(_e) = load_player(storage, world, entity, id).await {
            return send_infomsg(storage, socket_id, "Error Loading User.".into(), 1).await;
        }

        let name = {
            let lock = world.read().await;
            let username = lock.get::<&mut Account>(entity.0)?.username.clone();
            username
        };

        info!("Player {} with IP: {}, Logging in.", &name, address);

        {
            let mut names_lock = storage.player_emails.write().await;
            names_lock.insert(email, *entity);
        }

        return send_login_info(world, storage, entity, code, handshake, socket_id, name).await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_move(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
            || world.get_or_err::<Stunned>(entity).await?.0
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let data_pos = data.read::<Position>()?;

        if storage.bases.maps.get(&data_pos.map).is_none() || dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        let pos = world.get_or_err::<Position>(entity).await?;

        if data_pos != pos {
            //println!("Desync! {:?} {:?}", data_pos, pos);
            player_warp(world, storage, entity, &pos, false).await?;
            return Ok(());
        }

        let id = {
            let lock = world.read().await;
            let id = lock.get::<&Socket>(entity.0)?.id;
            id
        };

        return send_move_ok(
            storage,
            id,
            player_movement(world, storage, entity, dir).await?,
        )
        .await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_dir(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        {
            let lock = world.write().await;
            lock.get::<&mut Dir>(entity.0)?.0 = dir;
        }

        DataTaskToken::Dir(world.get_or_err::<Position>(entity).await?.map)
            .add_task(storage, dir_packet(*entity, dir)?)
            .await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_attack(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<AttackTimer>(entity).await?.0 > *storage.gettick.read().await
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let target = data.read::<Option<Entity>>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        if world.get_or_err::<Dir>(entity).await?.0 != dir {
            {
                let lock = world.write().await;
                lock.get::<&mut Dir>(entity.0)?.0 = dir;
            }
            DataTaskToken::Dir(world.get_or_err::<Position>(entity).await?.map)
                .add_task(storage, dir_packet(*entity, dir)?)
                .await?;
        };

        if let Some(target_entity) = target {
            if world.contains(&target_entity).await {
                if !player_combat(world, storage, entity, &target_entity).await? {
                    player_interact_object(world, storage, entity).await?;
                }
                {
                    let lock = world.write().await;
                    lock.get::<&mut AttackTimer>(entity.0)?.0 = *storage.gettick.read().await
                        + Duration::try_milliseconds(250).unwrap_or_default();
                }
            }
        } else {
            player_interact_object(world, storage, entity).await?;
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_useitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
            || world.get_or_err::<PlayerItemTimer>(entity).await?.itemtimer
                > *storage.gettick.read().await
        {
            return Ok(());
        }

        let slot = data.read::<u16>()?;

        {
            let lock = world.write().await;
            lock.get::<&mut PlayerItemTimer>(entity.0)?.itemtimer =
                *storage.gettick.read().await + Duration::try_milliseconds(250).unwrap_or_default();
        }

        return player_use_item(world, storage, entity, slot).await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_unequip(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
            || world.get_or_err::<PlayerItemTimer>(entity).await?.itemtimer
                > *storage.gettick.read().await
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= EQUIPMENT_TYPE_MAX {
            return Ok(());
        }

        {
            let lock = world.read().await;
            let val = lock.get::<&Equipment>(entity.0)?.items[slot].val;

            if val == 0 {
                return Ok(());
            }
        }

        if !player_unequip(world, storage, entity, slot).await? {
            send_fltalert(
                storage,
                {
                    let lock = world.read().await;
                    let id = lock.get::<&Socket>(entity.0)?.id;
                    id
                },
                "Could not unequiped. No inventory space.".into(),
                FtlType::Error,
            )
            .await?;
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_switchinvslot(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
            || world.get_or_err::<PlayerItemTimer>(entity).await?.itemtimer
                > *storage.gettick.read().await
        {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if oldslot >= MAX_INV || newslot >= MAX_INV || {
            let lock = world.read().await;
            let val = lock.get::<&Inventory>(entity.0)?.items[oldslot].val;
            val == 0
        } {
            return Ok(());
        }

        amount = amount.min({
            let lock = world.read().await;
            let val = lock.get::<&Inventory>(entity.0)?.items[oldslot].val;
            val
        });

        let mut itemold = {
            let lock = world.read().await;
            let old = lock.get::<&Inventory>(entity.0)?.items[oldslot];
            old
        };

        let (oldval, oldnum, newval, newnum) = {
            let lock = world.read().await;
            let newval = lock.get::<&Inventory>(entity.0)?.items[newslot].val;
            let newnum = lock.get::<&Inventory>(entity.0)?.items[newslot].num;
            let oldval = lock.get::<&Inventory>(entity.0)?.items[oldslot].val;
            let oldnum = lock.get::<&Inventory>(entity.0)?.items[oldslot].num;
            (oldval, oldnum, newval, newnum)
        };
        if newval > 0 {
            if newnum == oldnum {
                let take_amount = amount
                    - set_inv_slot(world, storage, entity, &mut itemold, newslot, amount).await?;
                if take_amount > 0 {
                    take_inv_itemslot(world, storage, entity, oldslot, take_amount).await?;
                }
            } else if oldval == amount {
                {
                    let lock = world.write().await;

                    let itemnew = lock.get::<&Inventory>(entity.0)?.items[newslot];
                    {
                        lock.get::<&mut Inventory>(entity.0)?.items[newslot] = itemold;
                        lock.get::<&mut Inventory>(entity.0)?.items[oldslot] = itemnew;
                    }
                }
                save_inv_item(world, storage, entity, newslot).await?;
                save_inv_item(world, storage, entity, oldslot).await?;
            } else {
                return send_fltalert(
                        storage,
                        {
                            let lock = world.read().await;
                            let id = lock.get::<&Socket>(entity.0)?.id;
                            id
                        },
                        "Can not swap slots with a different containing items unless you swap everything."
                            .into(),
                        FtlType::Error
                    ).await;
            }
        } else {
            {
                let lock = world.write().await;
                let itemnew = lock.get::<&Inventory>(entity.0)?.items[newslot];
                {
                    lock.get::<&mut Inventory>(entity.0)?.items[newslot] = itemold;
                    lock.get::<&mut Inventory>(entity.0)?.items[oldslot] = itemnew;
                }
            }
            save_inv_item(world, storage, entity, newslot).await?;
            save_inv_item(world, storage, entity, oldslot).await?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_pickup(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        let mut remove_id: Vec<(MapPosition, Entity)> = Vec::new();

        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
            || world
                .get_or_err::<PlayerMapTimer>(entity)
                .await?
                .mapitemtimer
                > *storage.gettick.read().await
        {
            return Ok(());
        }

        let mapids = get_maps_in_range(storage, &world.get_or_err::<Position>(entity).await?, 1);
        let mut full_message = false;

        for id in mapids {
            if let Some(x) = id.get() {
                let map = match storage.maps.get(&x) {
                    Some(map) => map,
                    None => continue,
                }
                .read()
                .await;

                // for the map base data when we need it.
                if storage
                    .bases
                    .maps
                    .get(&world.get_or_err::<Position>(entity).await?.map)
                    .is_none()
                {
                    continue;
                }
                let ids = map.itemids.clone();

                for i in ids {
                    let mut mapitems = world.get_or_err::<MapItem>(&i).await?;
                    if world
                        .get_or_err::<Position>(entity)
                        .await?
                        .checkdistance(mapitems.pos.map_offset(id.into()))
                        <= 1
                    {
                        if mapitems.item.num == 0 {
                            let rem =
                                player_give_vals(world, storage, entity, mapitems.item.val as u64)
                                    .await?;

                            {
                                let lock = world.write().await;
                                lock.get::<&mut MapItem>(i.0)?.item.val = rem as u16;
                            }

                            if rem == 0 {
                                remove_id.push((x, i));
                            }
                        } else {
                            //let amount = mapitems.item.val;

                            let (is_less, amount, start) =
                                check_inv_partial_space(world, storage, entity, &mut mapitems.item)
                                    .await?;

                            //if passed then we only get partial of the map item.
                            if is_less {
                                give_inv_item(world, storage, entity, &mut mapitems.item).await?;

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
                                    ).await?;
                                    let lock = world.write().await;
                                    lock.get::<&mut MapItem>(i.0)?.item.val = start - amount;
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
                                    )
                                    .await?;

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
            )
            .await?;
        }

        for (mappos, entity) in remove_id.iter_mut() {
            if let Some(map) = storage.maps.get(mappos) {
                let pos = world.get_or_err::<MapItem>(entity).await?.pos;
                let mut storage_mapitems = storage.map_items.write().await;
                if storage_mapitems.contains_key(&pos) {
                    storage_mapitems.swap_remove(&pos);
                }
                map.write().await.remove_item(*entity);
                DataTaskToken::EntityUnload(*mappos)
                    .add_task(storage, unload_entity_packet(*entity)?)
                    .await?;
            }
        }
        {
            let lock = world.write().await;
            lock.get::<&mut PlayerMapTimer>(entity.0)?.mapitemtimer =
                *storage.gettick.read().await + Duration::try_milliseconds(100).unwrap_or_default();
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_dropitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if slot >= MAX_INV || amount == 0 {
            return Ok(());
        }

        let slot_val = {
            let lock = world.read().await;
            let val = lock.get::<&Inventory>(entity.0)?.items[slot].val;
            val
        };

        if slot_val == 0 {
            return Ok(());
        }

        amount = amount.min(slot_val);

        let item_data = {
            let lock = world.read().await;
            let item = lock.get::<&Inventory>(entity.0)?.items[slot];
            item
        };

        //make sure it exists first.
        if !storage
            .bases
            .maps
            .contains_key(&world.get_or_err::<Position>(entity).await?.map)
        {
            return Err(AscendingError::Unhandled(Box::new(Backtrace::capture())));
        }

        if try_drop_item(
            world,
            storage,
            DropItem {
                index: item_data.num,
                amount,
                pos: world.get_or_err::<Position>(entity).await?,
            },
            match world.get_or_err::<UserAccess>(entity).await? {
                UserAccess::Admin => None,
                _ => Some(
                    *storage.gettick.read().await
                        + Duration::try_milliseconds(600000).unwrap_or_default(),
                ),
            },
            Some(
                *storage.gettick.read().await
                    + Duration::try_milliseconds(5000).unwrap_or_default(),
            ),
            Some(*entity),
        )
        .await?
        {
            take_inv_itemslot(world, storage, entity, slot, amount).await?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_deleteitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= MAX_INV {
            return Ok(());
        }

        let slot_val = {
            let lock = world.read().await;
            let val = lock.get::<&Inventory>(entity.0)?.items[slot].val;
            val
        };

        if slot_val == 0 {
            return Ok(());
        }

        let val = slot_val;
        take_inv_itemslot(world, storage, entity, slot, val).await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_switchstorageslot(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_bank()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
        {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if oldslot >= MAX_STORAGE || newslot >= MAX_STORAGE {
            return Ok(());
        }

        let (mut olditem, newitem) = {
            let lock = world.read().await;
            let olditem = lock.get::<&PlayerStorage>(entity.0)?.items[oldslot];
            let newitem = lock.get::<&PlayerStorage>(entity.0)?.items[newslot];
            (olditem, newitem)
        };

        if olditem.val == 0 {
            return Ok(());
        }

        amount = amount.min(olditem.val);

        if newitem.val > 0 {
            if newitem.num == olditem.num {
                let take_amount = amount
                    - set_storage_slot(world, storage, entity, &mut olditem, newslot, amount)
                        .await?;
                if take_amount > 0 {
                    take_storage_itemslot(world, storage, entity, oldslot, take_amount).await?;
                }
            } else if olditem.val == amount {
                {
                    let lock = world.write().await;
                    lock.get::<&mut PlayerStorage>(entity.0)?.items[newslot] = olditem;
                    lock.get::<&mut PlayerStorage>(entity.0)?.items[oldslot] = newitem;
                }
                save_storage_item(world, storage, entity, newslot).await?;
                save_storage_item(world, storage, entity, oldslot).await?;
            } else {
                return send_fltalert(
                        storage,
                        {
                            let lock = world.read().await;
                            let id = lock.get::<&Socket>(entity.0)?.id;
                            id },
                        "Can not swap slots with a different containing items unless you swap everything."
                            .into(),
                        FtlType::Error
                    ).await;
            }
        } else {
            {
                let lock = world.write().await;
                lock.get::<&mut PlayerStorage>(entity.0)?.items[newslot] = olditem;
                lock.get::<&mut PlayerStorage>(entity.0)?.items[oldslot] = newitem;
            }
            save_storage_item(world, storage, entity, newslot).await?;
            save_storage_item(world, storage, entity, oldslot).await?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_deletestorageitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_bank()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= MAX_STORAGE {
            return Ok(());
        }

        let slot_val = {
            let lock = world.read().await;
            let val = lock.get::<&PlayerStorage>(entity.0)?.items[slot].val;
            val
        };

        if slot_val == 0 {
            return Ok(());
        }

        take_storage_itemslot(world, storage, entity, slot, slot_val).await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_deposititem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_bank()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
        {
            return Ok(());
        }

        let inv_slot = data.read::<u16>()? as usize;
        let bank_slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if bank_slot >= MAX_STORAGE || inv_slot >= MAX_INV {
            return Ok(());
        }

        let (mut inv_item, bank_item) = {
            let lock = world.read().await;
            let item_data = lock.get::<&Inventory>(entity.0)?.items[inv_slot];
            let bank_data = lock.get::<&PlayerStorage>(entity.0)?.items[bank_slot];
            (item_data, bank_data)
        };

        if inv_item.val == 0 {
            return Ok(());
        }

        if inv_item.val > amount {
            inv_item.val = amount;
        };

        if bank_item.val == 0 {
            {
                let lock = world.write().await;
                lock.get::<&mut PlayerStorage>(entity.0)?.items[bank_slot] = inv_item;
            }
            save_storage_item(world, storage, entity, bank_slot).await?;
            take_inv_itemslot(world, storage, entity, inv_slot, amount).await?;
        } else {
            let (is_less, amount, _started) =
                check_storage_partial_space(world, storage, entity, &mut inv_item).await?;

            if is_less {
                give_storage_item(world, storage, entity, &mut inv_item).await?;
                take_inv_itemslot(world, storage, entity, inv_slot, amount).await?;
            } else {
                send_message(
                    world,
                    storage,
                    entity,
                    "You do not have enough slot to deposit this item!".into(),
                    String::new(),
                    MessageChannel::Private,
                    None,
                )
                .await?;
            }
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_withdrawitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_bank()
            || world.get_or_err::<Attacking>(entity).await?.0
            || world.get_or_err::<Stunned>(entity).await?.0
        {
            return Ok(());
        }

        let inv_slot = data.read::<u16>()? as usize;
        let bank_slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if bank_slot >= MAX_STORAGE || inv_slot >= MAX_INV {
            return Ok(());
        }

        let (mut bank_item, inv_item) = {
            let lock = world.read().await;
            let bank_item = lock.get::<&PlayerStorage>(entity.0)?.items[bank_slot];
            let inv_item = lock.get::<&Inventory>(entity.0)?.items[inv_slot];
            (bank_item, inv_item)
        };

        if bank_item.val == 0 {
            return Ok(());
        }

        if bank_item.val > amount {
            bank_item.val = amount;
        };

        if { inv_item.val } == 0 {
            {
                let lock = world.write().await;
                lock.get::<&mut Inventory>(entity.0)?.items[inv_slot] = bank_item;
            }
            save_inv_item(world, storage, entity, inv_slot).await?;
            take_storage_itemslot(world, storage, entity, bank_slot, amount).await?;
        } else {
            let (is_less, amount, _started) =
                check_inv_partial_space(world, storage, entity, &mut bank_item).await?;

            if is_less {
                give_inv_item(world, storage, entity, &mut bank_item).await?;
                take_storage_itemslot(world, storage, entity, bank_slot, amount).await?;
            } else {
                send_message(
                    world,
                    storage,
                    entity,
                    "You do not have enough slot to withdraw this item!".into(),
                    String::new(),
                    MessageChannel::Private,
                    None,
                )
                .await?;
            }
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_message(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        let mut usersocket: Option<usize> = None;

        if !world.get_or_err::<Death>(entity).await?.is_alive() {
            return Ok(());
        }

        let channel: MessageChannel = data.read()?;

        let msg = data.read::<String>()?;
        let name = data.read::<String>()?;

        let (username, socket_id) = {
            let lock = world.read().await;
            let id = lock.get::<&Socket>(entity.0)?.id;
            let username = lock.get::<&Account>(entity.0)?.username.clone();
            (username, id)
        };

        if msg.len() >= 256 {
            return send_fltalert(
                storage,
                socket_id,
                "Your message is too long. (256 character limit)".into(),
                FtlType::Error,
            )
            .await;
        }

        let head = match channel {
            MessageChannel::Map => {
                format!("[Map] {}:", username)
            }
            MessageChannel::Global => {
                format!("[Global] {}:", username)
            }
            MessageChannel::Trade => {
                format!("[Trade] {}:", username)
            }
            MessageChannel::Party => {
                format!("[Party] {}:", username)
            }
            MessageChannel::Private => {
                if name.is_empty() {
                    return Ok(());
                }

                if name == username {
                    return send_fltalert(
                        storage,
                        socket_id,
                        "You cannot send messages to yourself".into(),
                        FtlType::Error,
                    )
                    .await;
                }

                usersocket = match storage.player_usernames.read().await.get(&name) {
                    Some(id) => {
                        let lock = world.read().await;
                        let socket_ref = lock.get::<&Socket>(id.0);
                        if let Ok(socket) = socket_ref {
                            Some(socket.id)
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
                        )
                        .await;
                    }
                };

                format!("[Private] {}:", username)
            }
            MessageChannel::Guild => {
                format!("[Guild] {}:", username)
            }
            MessageChannel::Help => {
                format!("[Help] {}:", username)
            }
            MessageChannel::Quest => "".into(),
            MessageChannel::Npc => "".into(),
        };

        return send_message(world, storage, entity, msg, head, channel, usersocket).await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_command(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(_p) = storage.player_ids.read().await.get(entity) {
        let command = data.read::<Command>()?;

        match command {
            Command::KickPlayer => {}
            Command::KickPlayerByName(name) => {
                debug!("Kicking Player {:?}", name);
            }
            Command::WarpTo(pos) => {
                debug!("Warping to {:?}", pos);
                player_warp(world, storage, entity, &pos, false).await?;
            }
            Command::SpawnNpc(index, pos) => {
                debug!("Spawning NPC {index} on {:?}", pos);
                if let Some(mapdata) = storage.maps.get(&pos.map) {
                    let mut data = mapdata.write().await;
                    if let Ok(Some(id)) = storage.add_npc(world, index as u64).await {
                        data.add_npc(id);
                        spawn_npc(world, pos, None, id).await?;
                    }
                }
            }
            Command::Trade => {
                let target = world.get_or_err::<PlayerTarget>(entity).await?.0;
                if let Some(target_entity) = target
                    && world.contains(&target_entity).await
                {
                    //init_trade(world, storage, entity, &target_entity)?;
                    if world
                        .get_or_err::<TradeRequestEntity>(entity)
                        .await?
                        .requesttimer
                        <= *storage.gettick.read().await
                        && can_target(
                            world.get_or_err::<Position>(entity).await?,
                            world.get_or_err::<Position>(&target_entity).await?,
                            world.get_or_err::<Death>(&target_entity).await?,
                            1,
                        )
                        && can_trade(world, storage, &target_entity).await?
                    {
                        send_traderequest(world, storage, entity, &target_entity).await?;
                        {
                            let lock = world.write().await;
                            if let Ok(mut traderequest) =
                                lock.get::<&mut TradeRequestEntity>(entity.0)
                            {
                                traderequest.entity = Some(target_entity);
                                traderequest.requesttimer = *storage.gettick.read().await
                                    + Duration::try_milliseconds(60000).unwrap_or_default();
                                // 1 Minute
                            }
                            if let Ok(mut traderequest) =
                                lock.get::<&mut TradeRequestEntity>(target_entity.0)
                            {
                                traderequest.entity = Some(*entity);
                                traderequest.requesttimer = *storage.gettick.read().await
                                    + Duration::try_milliseconds(60000).unwrap_or_default();
                                // 1 Minute
                            }

                            drop(lock);
                        }
                        send_message(
                            world,
                            storage,
                            entity,
                            "Trade Request Sent".into(),
                            String::new(),
                            MessageChannel::Private,
                            None,
                        )
                        .await?;
                    } else {
                        send_message(
                            world,
                            storage,
                            entity,
                            "Player is busy".into(),
                            String::new(),
                            MessageChannel::Private,
                            None,
                        )
                        .await?;
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
                    )
                    .await?;
                }
            }
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_settarget(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(_p) = storage.player_ids.read().await.get(entity) {
        let target = data.read::<Option<Entity>>()?;

        if let Some(target_entity) = target {
            if !world.contains(&target_entity).await {
                return Ok(());
            }
        }

        let lock = world.write().await;
        lock.get::<&mut PlayerTarget>(entity.0)?.0 = target;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_closestorage(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_bank()
        {
            return Ok(());
        }

        {
            let lock = world.write().await;
            *lock.get::<&mut IsUsingType>(entity.0)? = IsUsingType::None;
        }
        send_clearisusingtype(world, storage, entity).await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_closeshop(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_instore()
        {
            return Ok(());
        }

        {
            let lock = world.write().await;
            *lock.get::<&mut IsUsingType>(entity.0)? = IsUsingType::None;
        }
        send_clearisusingtype(world, storage, entity).await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_closetrade(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_trading()
        {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity).await? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(&target_entity).await {
            return Ok(());
        }

        close_trade(world, storage, entity).await?;
        close_trade(world, storage, &target_entity).await?;

        {
            let lock = world.write().await;
            *lock.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
            *lock.get::<&mut TradeRequestEntity>(target_entity.0)? = TradeRequestEntity::default();
        }

        return send_message(
            world,
            storage,
            &target_entity,
            format!(
                "{} has cancelled the trade",
                world.cloned_get_or_err::<Account>(entity).await?.username
            ),
            String::new(),
            MessageChannel::Private,
            None,
        )
        .await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_buy_item(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_instore()
        {
            return Ok(());
        }
        let shop_index =
            if let IsUsingType::Store(shop) = world.get_or_err::<IsUsingType>(entity).await? {
                shop
            } else {
                return Ok(());
            };
        let slot = data.read::<u16>()?;

        let shopdata = storage.bases.shops[shop_index as usize].clone();

        let player_money = world.get_or_err::<Money>(entity).await?.vals;
        if player_money < shopdata.item[slot as usize].price {
            return send_message(
                world,
                storage,
                entity,
                "You do not have enough money".into(),
                String::new(),
                MessageChannel::Private,
                None,
            )
            .await;
        }

        let mut item = Item {
            num: shopdata.item[slot as usize].index as u32,
            val: shopdata.item[slot as usize].amount,
            ..Default::default()
        };

        if check_inv_space(world, storage, entity, &mut item).await? {
            give_inv_item(world, storage, entity, &mut item).await?;
            player_take_vals(world, storage, entity, shopdata.item[slot as usize].price).await?;
        } else {
            return send_message(
                world,
                storage,
                entity,
                "You do not have enough space in your inventory".into(),
                String::new(),
                MessageChannel::Private,
                None,
            )
            .await;
        }

        return send_message(
            world,
            storage,
            entity,
            "You have successfully bought an item!".into(),
            String::new(),
            MessageChannel::Private,
            None,
        )
        .await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_sellitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_instore()
        {
            return Ok(());
        }
        let _shop_index =
            if let IsUsingType::Store(shop) = world.get_or_err::<IsUsingType>(entity).await? {
                shop
            } else {
                return Ok(());
            };
        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()?;

        if slot >= MAX_INV {
            return Ok(());
        }

        let inv_item = {
            let lock = world.read().await;
            let inv_item = lock.get::<&Inventory>(entity.0)?.items[slot];
            inv_item
        };

        if inv_item.val == 0 {
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
        take_inv_itemslot(world, storage, entity, slot, amount).await?;
        player_give_vals(world, storage, entity, total_price).await?;

        return send_message(
            world,
            storage,
            entity,
            format!("You have sold an item for {}!", total_price),
            String::new(),
            MessageChannel::Private,
            None,
        )
        .await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_addtradeitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_trading()
        {
            return Ok(());
        }
        if world.get_or_err::<TradeStatus>(entity).await? != TradeStatus::None {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity).await? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(&target_entity).await {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u16>()? as u64;

        if slot >= MAX_INV {
            return Ok(());
        }

        let mut inv_item = {
            let lock = world.read().await;
            let inv_item = lock.get::<&Inventory>(entity.0)?.items[slot];
            inv_item
        };

        if inv_item.val == 0 {
            return Ok(());
        }

        let base = &storage.bases.items[inv_item.num as usize];
        if base.stackable && amount > base.stacklimit as u64 {
            amount = base.stacklimit as u64
        }

        // Make sure it does not exceed the amount player have
        let (inv_count, trade_count) = {
            let lock = world.read().await;
            let inv = lock.get::<&Inventory>(entity.0)?;
            let trade = lock.get::<&TradeItem>(entity.0)?;

            (
                count_inv_item(inv_item.num, &inv.items),
                count_trade_item(inv_item.num, &trade.items),
            )
        };

        if trade_count + amount > inv_count {
            amount = inv_count.saturating_sub(trade_count);
        }
        if amount == 0 {
            return Ok(());
        }
        inv_item.val = amount as u16;

        // Add the item on trade list
        let trade_slot_list = give_trade_item(world, storage, entity, &mut inv_item).await?;

        for slot in trade_slot_list.iter() {
            send_updatetradeitem(world, storage, entity, entity, *slot as u16).await?;
            send_updatetradeitem(world, storage, entity, &target_entity, *slot as u16).await?;
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_removetradeitem(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_trading()
        {
            return Ok(());
        }
        if world.get_or_err::<TradeStatus>(entity).await? != TradeStatus::None {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity).await? {
                entity
            } else {
                return Ok(());
            };

        if !world.contains(&target_entity).await {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let mut amount = data.read::<u64>()?;

        let trade_item = world.cloned_get_or_err::<TradeItem>(entity).await?.items[slot];

        if slot >= MAX_TRADE_SLOT || trade_item.val == 0 {
            return Ok(());
        }
        amount = amount.min(trade_item.val as u64);

        {
            let lock = world.write().await;
            let trade_slot = lock.get::<&mut TradeItem>(entity.0);
            if let Ok(mut tradeitem) = trade_slot {
                tradeitem.items[slot].val = tradeitem.items[slot].val.saturating_sub(amount as u16);
                if tradeitem.items[slot].val == 0 {
                    tradeitem.items[slot] = Item::default();
                }
            }
        }

        send_updatetradeitem(world, storage, entity, entity, slot as u16).await?;
        send_updatetradeitem(world, storage, entity, &target_entity, slot as u16).await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_updatetrademoney(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_trading()
        {
            return Ok(());
        }
        if world.get_or_err::<TradeStatus>(entity).await? != TradeStatus::None {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity).await? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(&target_entity).await {
            return Ok(());
        }

        let money = world.get_or_err::<Money>(entity).await?.vals;
        let amount = data.read::<u64>()?.min(money);

        {
            let lock = world.write().await;
            lock.get::<&mut TradeMoney>(entity.0)?.vals = amount;
        }
        send_updatetrademoney(world, storage, entity, &target_entity).await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_submittrade(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || !world.get_or_err::<IsUsingType>(entity).await?.is_trading()
        {
            return Ok(());
        }

        let target_entity =
            if let IsUsingType::Trading(entity) = world.get_or_err::<IsUsingType>(entity).await? {
                entity
            } else {
                return Ok(());
            };
        if !world.contains(&target_entity).await {
            return Ok(());
        }

        let entity_status = world.get_or_err::<TradeStatus>(entity).await?;
        let target_status = world.get_or_err::<TradeStatus>(&target_entity).await?;

        match entity_status {
            TradeStatus::None => {
                let lock = world.write().await;
                *lock.get::<&mut TradeStatus>(entity.0)? = TradeStatus::Accepted;
            }
            TradeStatus::Accepted => {
                if target_status == TradeStatus::Accepted {
                    {
                        let lock = world.write().await;
                        *lock.get::<&mut TradeStatus>(entity.0)? = TradeStatus::Submitted;
                    }
                } else if target_status == TradeStatus::Submitted {
                    {
                        let lock = world.write().await;
                        *lock.get::<&mut TradeStatus>(entity.0)? = TradeStatus::Submitted;
                    }
                    if !process_player_trade(world, storage, entity, &target_entity).await? {
                        send_message(
                            world,
                            storage,
                            entity,
                            "One of you does not have enough inventory slot to proceed with the trade".into(),
                            String::new(),
                            MessageChannel::Private,
                            None,
                        ).await?;
                        send_message(
                            world,
                            storage,
                            &target_entity,
                            "One of you does not have enough inventory slot to proceed with the trade".into(),
                            String::new(),
                            MessageChannel::Private,
                            None,
                        ).await?;
                    }
                    close_trade(world, storage, entity).await?;
                    close_trade(world, storage, &target_entity).await?;
                    return Ok(());
                }
            }
            _ => {}
        }
        send_tradestatus(
            world,
            storage,
            entity,
            &world.get_or_err::<TradeStatus>(entity).await?,
            &target_status,
        )
        .await?;
        send_tradestatus(
            world,
            storage,
            &target_entity,
            &target_status,
            &world.get_or_err::<TradeStatus>(entity).await?,
        )
        .await?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_accepttrade(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
        {
            return Ok(());
        }

        let target_entity = match world.get_or_err::<TradeRequestEntity>(entity).await?.entity {
            Some(entity) => entity,
            None => return Ok(()),
        };
        {
            let lock = world.write().await;
            *lock.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
        }

        if !world.contains(&target_entity).await {
            return Ok(());
        }

        let trade_entity = match world
            .get_or_err::<TradeRequestEntity>(&target_entity)
            .await?
            .entity
        {
            Some(entity) => entity,
            None => return Ok(()),
        };
        if trade_entity != *entity
            || world
                .get_or_err::<IsUsingType>(&trade_entity)
                .await?
                .inuse()
        {
            return Ok(());
        }

        {
            let lock = world.write().await;
            *lock.get::<&mut TradeStatus>(entity.0)? = TradeStatus::None;
            *lock.get::<&mut TradeStatus>(trade_entity.0)? = TradeStatus::None;
            *lock.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
            *lock.get::<&mut TradeRequestEntity>(target_entity.0)? = TradeRequestEntity::default();
        }
        send_tradestatus(
            world,
            storage,
            entity,
            &world.get_or_err::<TradeStatus>(entity).await?,
            &world.get_or_err::<TradeStatus>(&trade_entity).await?,
        )
        .await?;
        send_tradestatus(
            world,
            storage,
            &trade_entity,
            &world.get_or_err::<TradeStatus>(&trade_entity).await?,
            &world.get_or_err::<TradeStatus>(entity).await?,
        )
        .await?;

        return init_trade(world, storage, entity, &target_entity).await;
    }

    Err(AscendingError::InvalidSocket)
}

pub async fn handle_declinetrade(
    world: &GameWorld,
    storage: &GameStore,
    _data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(entity) = storage.player_ids.read().await.get(entity) {
        if !world.get_or_err::<Death>(entity).await?.is_alive()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
        {
            return Ok(());
        }

        let target_entity = match world.get_or_err::<TradeRequestEntity>(entity).await?.entity {
            Some(entity) => entity,
            None => return Ok(()),
        };
        {
            let lock = world.write().await;
            *lock.get::<&mut TradeRequestEntity>(entity.0)? = TradeRequestEntity::default();
        }

        if world.contains(&target_entity).await {
            let trade_entity = match world
                .get_or_err::<TradeRequestEntity>(&target_entity)
                .await?
                .entity
            {
                Some(entity) => entity,
                None => return Ok(()),
            };
            if trade_entity == *entity {
                let lock = world.write().await;
                *lock.get::<&mut TradeRequestEntity>(target_entity.0)? =
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
            )
            .await?;
        }

        return send_message(
            world,
            storage,
            entity,
            "Trade Request has been declined".into(),
            String::new(),
            MessageChannel::Private,
            None,
        )
        .await;
    }

    Err(AscendingError::InvalidSocket)
}
