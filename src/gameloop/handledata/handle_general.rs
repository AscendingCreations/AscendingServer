use chrono::Duration;
use log::{debug, info};
use mmap_bytey::MByteBuffer;
use rand::distr::{Alphanumeric, SampleString};

use crate::{
    containers::{
        Entity, GlobalKey, IsUsingType, PlayerConnectionTimer, Socket, Storage, TradeRequestEntity,
        World,
    },
    gametypes::*,
    items::Item,
    maps::{can_target, spawn_npc},
    players::{
        can_trade, check_inv_space, close_trade, give_inv_item, player_give_vals, player_take_vals,
        player_warp, reconnect_player, send_reconnect_info, send_tls_reconnect, take_inv_itemslot,
    },
    socket::{
        MByteBufferExt, send_clear_data, send_clearisusingtype, send_fltalert, send_gameping,
        send_message, send_traderequest,
    },
    time_ext::MyInstant,
};

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

    let mut usersocket: Option<usize> = None;

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
                        Some(p_data.try_lock()?.socket.id)
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
            debug!("Kicking Player {name:?}");
        }
        Command::WarpTo(pos) => {
            debug!("Warping to {pos:?}");
            player_warp(world, storage, entity, &pos, false)?;
        }
        Command::SpawnNpc(index, pos) => {
            debug!("Spawning NPC {index} on {pos:?}");
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
                format!("{username} has cancelled the trade"),
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
            format!("You have sold an item for {total_price}!"),
            String::new(),
            MessageChannel::Private,
            None,
        )?;
    }
    Ok(())
}

pub fn handle_login_ok(
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
        let connection_code = data.read_str()?;

        let mut p_data = p_data.try_lock()?;
        let mut player_codes = storage.player_code.borrow_mut();

        for code in p_data.relogin_code.code.iter() {
            if *code != connection_code {
                player_codes.swap_remove(code);
            }
        }

        p_data.relogin_code.code.clear();
        p_data.relogin_code.code.insert(connection_code);

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_tls_reconnect(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    _entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let connection_code = data.read_str()?;
    let code = storage.player_code.borrow().get(&connection_code).cloned();

    if let Some(entity) = code
        && let Some(Entity::Player(p_data)) = world.get_opt_entity(entity)
    {
        let code = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let handshake = Alphanumeric.sample_string(&mut rand::rng(), 32);

        if let Some(client) = storage.server.borrow().clients.get(socket_id.id) {
            client.borrow_mut().entity = Some(entity);
        }

        {
            p_data.try_lock()?.socket.tls_id = socket_id.id;
        }

        return send_tls_reconnect(world, storage, entity, code, handshake);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_tls_handshake(
    world: &mut World,
    _storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    _socket_id: SocketID,
) -> Result<()> {
    let handshake = data.read::<String>()?;

    let entity = match entity {
        Some(e) => e,
        None => return Err(AscendingError::InvalidSocket),
    };

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity)
        && p_data.try_lock()?.login_handshake.handshake == handshake
    {
        println!("TLS Reconnect Complete");

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_disconnect(
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
        let _ = data.read::<u32>()?;

        let can_exit = {
            let p_data = p_data.try_lock()?;

            if !p_data.combat.in_combat {
                let socket = &p_data.socket;

                if let Some(client) = storage.server.borrow_mut().clients.get_mut(socket.id) {
                    client.borrow_mut().entity = None;
                }

                if let Some(client) = storage.server.borrow_mut().clients.get_mut(socket.tls_id) {
                    client.borrow_mut().entity = None;
                }
            }

            p_data.combat.in_combat
        };

        if !can_exit {
            storage
                .player_timeout
                .borrow_mut()
                .insert(entity, PlayerConnectionTimer(MyInstant::recent()));
        }
    }

    Ok(())
}

pub fn handle_reconnect(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    _entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let connection_code = data.read_str()?;
    let code = storage.player_code.borrow().get(&connection_code).cloned();

    if let Some(old_entity) = code
        && let Some(Entity::Player(p_data)) = world.get_opt_entity(old_entity)
    {
        if !storage.disconnected_player.borrow().contains(&old_entity) {
            return Err(AscendingError::InvalidSocket);
        }

        let socket = if let Some(client) = storage.server.borrow().clients.get(socket_id.id) {
            let brw_client = client.borrow();
            Socket::new(usize::MAX, socket_id.id, brw_client.addr.to_string())?
        } else {
            return Err(AscendingError::InvalidSocket);
        };

        let (socket_token, address) = (socket.tls_id, socket.addr.clone());

        reconnect_player(world, storage, old_entity, socket)?;

        let name = { p_data.try_lock()?.account.username.clone() };

        info!(
            "Player {} with IP: {}, Reconnecting on handle_reconnect .",
            &name, address
        );

        let code = Alphanumeric.sample_string(&mut rand::rng(), 32);
        let handshake = Alphanumeric.sample_string(&mut rand::rng(), 32);

        {
            let _ = storage
                .disconnected_player
                .borrow_mut()
                .swap_remove(&old_entity);
        }

        {
            storage
                .hand_shakes
                .borrow_mut()
                .insert(handshake.clone(), old_entity);
        }

        send_clear_data(world, storage, old_entity)?;

        return send_reconnect_info(
            world,
            storage,
            old_entity,
            code,
            handshake,
            socket_token,
            name,
        );
    }

    Err(AscendingError::InvalidSocket)
}
