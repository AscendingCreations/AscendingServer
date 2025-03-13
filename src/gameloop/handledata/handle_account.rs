use chrono::Duration;
use log::info;
use mio::Token;
use mmap_bytey::MByteBuffer;
use rand::distr::{Alphanumeric, SampleString};
use regex::Regex;

use crate::{
    containers::{Entity, GlobalKey, PlayerConnectionTimer, Socket, Storage, World},
    gametypes::*,
    players::{
        is_name_acceptable, is_password_acceptable, joingame, reconnect_player, send_login_info,
        send_reconnect_info,
    },
    socket::{ClientState, disconnect, send_codes, send_infomsg, send_myindex},
    sql::{check_existance, find_player, load_player, new_player},
};

use super::SocketID;

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
        Socket::new(Token(0), socket_id.id, brw_client.addr.to_string())?
    } else {
        return Err(AscendingError::InvalidSocket);
    };

    if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
        return send_infomsg(
            storage,
            socket.tls_id,
            "Client needs to be updated.".into(),
            1,
        );
    }

    let email_regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )?;

    if !username.chars().all(is_name_acceptable) || !password.chars().all(is_password_acceptable) {
        return send_infomsg(
            storage,
            socket.tls_id,
            "Username or Password contains unaccepted Characters".into(),
            0,
        );
    }

    if username.len() >= 64 {
        return send_infomsg(
            storage,
            socket.tls_id,
            "Username has too many Characters, 64 Characters Max".into(),
            0,
        );
    }

    if password.len() >= 128 {
        return send_infomsg(
            storage,
            socket.tls_id,
            "Password has too many Characters, 128 Characters Max".into(),
            0,
        );
    }

    if !email_regex.is_match(&email) || sprite_id >= 6 {
        return send_infomsg(
            storage,
            socket.tls_id,
            "Email must be an actual email.".into(),
            0,
        );
    }

    match check_existance(storage, &username, &email) {
        Ok(i) => match i {
            0 => {}
            1 => {
                return send_infomsg(
                    storage,
                    socket.tls_id,
                    "Username Exists. Please try Another.".into(),
                    0,
                );
            }
            2 => {
                return send_infomsg(
                    storage,
                    socket.tls_id,
                    "Email Already Exists. Please Try Another.".into(),
                    0,
                );
            }
            _ => return Err(AscendingError::RegisterFail),
        },
        Err(_) => return Err(AscendingError::UserNotFound),
    }

    match new_player(storage, username.clone(), email, password, &socket) {
        Ok(uid) => {
            let code = Alphanumeric.sample_string(&mut rand::rng(), 32);
            let handshake = Alphanumeric.sample_string(&mut rand::rng(), 32);

            // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
            let entity =
                storage.add_player_data(world, code.clone(), handshake.clone(), socket.clone())?;

            if let Some(client) = storage.server.borrow_mut().clients.get_mut(&socket.tls_id) {
                client.borrow_mut().entity = Some(entity);
            }

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

            if let Some(client) = storage.server.borrow_mut().clients.get_mut(&socket.tls_id) {
                client.borrow_mut().entity = Some(entity);
            }

            let tick = *storage.gettick.borrow();

            storage.player_timeout.borrow_mut().insert(
                entity,
                PlayerConnectionTimer(tick + Duration::try_milliseconds(60000).unwrap_or_default()),
            );

            info!(
                "New Player {} with IP {}, Logging in.",
                &username, &socket.addr
            );

            send_myindex(storage, socket.tls_id, entity)?;
            send_codes(world, storage, entity, code, handshake)
        }
        Err(_) => send_infomsg(
            storage,
            socket.tls_id,
            "There was an Issue Creating the player account. Please Contact Support.".into(),
            0,
        ),
    }
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

                p_data.socket.id = socket_id.id;

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
        Socket::new(Token(0), socket_id.id, brw_client.addr.to_string())?
    } else {
        return Err(AscendingError::InvalidSocket);
    };

    if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
        return send_infomsg(
            storage,
            socket.tls_id,
            "Client needs to be updated.".into(),
            1,
        );
    }

    if username.len() >= 64 || password.len() >= 128 {
        return send_infomsg(
            storage,
            socket.tls_id,
            "Account does not Exist or Password is not Correct.".into(),
            0,
        );
    }

    let id = match find_player(storage, &username, &password)? {
        Some(id) => id,
        None => {
            return send_infomsg(
                storage,
                socket.tls_id,
                "Account does not Exist or Password is not Correct.".into(),
                1,
            );
        }
    };

    // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
    let code = Alphanumeric.sample_string(&mut rand::rng(), 32);
    let handshake = Alphanumeric.sample_string(&mut rand::rng(), 32);
    let mut send_reconnect = None;
    let mut disconnect_player = None;
    let mut unload_socket = None;

    if let Some(old_entity) = storage.player_code.borrow().get(&reconnect_code) {
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(*old_entity) {
            if storage.disconnected_player.borrow().contains(old_entity) {
                // Character is on disconnected list
                reconnect_player(world, storage, *old_entity, socket.clone())?;

                let name = { p_data.try_lock()?.account.username.clone() };

                info!(
                    "Player {} with IP: {}, Reconnecting from disconnected player.",
                    &name, socket.addr
                );

                {
                    let _ = storage
                        .disconnected_player
                        .borrow_mut()
                        .swap_remove(old_entity);
                }

                send_reconnect = Some((*old_entity, name, socket.tls_id));
            } else {
                // Connected in same code but not disconnected
                let old_code = { p_data.try_lock()?.relogin_code.clone() };

                // if old code is empty means they did get unloaded just not all the way for some reason.
                if old_code.code.is_empty() {
                    let name = { p_data.try_lock()?.account.username.clone() };

                    let _ = storage.player_names.borrow_mut().remove(&name);
                } else if !reconnect_code.is_empty() && old_code.code.contains(&reconnect_code) {
                    let p_data = p_data.try_lock()?;

                    if p_data.socket.tls_id != socket.tls_id {
                        disconnect_player = Some(*old_entity);
                        unload_socket = Some((p_data.socket.tls_id, p_data.socket.id))
                    } else {
                        let name = p_data.account.username.clone();

                        info!(
                            "Player {} with IP: {}, Reconnecting not in disconnected player.",
                            &name, &p_data.socket.addr
                        );

                        send_reconnect = Some((*old_entity, name, p_data.socket.id));
                    }
                } else {
                    return send_infomsg(storage, socket.tls_id, "Error Loading User.".into(), 1);
                }
            }
        }
    }

    if let Some(old_entity) = disconnect_player {
        disconnect(old_entity, world, storage)?;
    }

    if let Some((old_entity, name, socket_token)) = send_reconnect {
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

    if let Some((tls_socket, non_tls_socket)) = unload_socket {
        if let Some(client) = storage.server.borrow_mut().clients.get_mut(&tls_socket) {
            client.borrow_mut().state = ClientState::Closing;
        }
        if let Some(client) = storage.server.borrow_mut().clients.get_mut(&non_tls_socket) {
            client.borrow_mut().state = ClientState::Closing;
        }
    }

    let user_entity = world.entities.iter().find_map(|(entity, data)| {
        if let Entity::Player(p_data) = data {
            if let Ok(player) = p_data.try_lock() {
                if player.account.username == username {
                    Some(entity)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    });

    // This check is in case the account is connected on different entity
    if let Some(old_entity) = user_entity {
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(old_entity) {
            let old_code = { p_data.try_lock()?.relogin_code.clone() };

            // if old code is empty means they did get unloaded just not all the way for some reason.
            if old_code.code.is_empty() {
                let name = { p_data.try_lock()?.account.username.clone() };

                let _ = storage.player_names.borrow_mut().remove(&name);
            } else if !reconnect_code.is_empty() && old_code.code.contains(&reconnect_code) {
                disconnect(old_entity, world, storage)?;
            } else {
                return send_infomsg(storage, socket.tls_id, "Error Loading User.".into(), 1);
            }
        }
    }

    let entity = storage.add_player_data(world, code.clone(), handshake.clone(), socket.clone())?;

    if let Err(_e) = load_player(storage, world, entity, id) {
        return send_infomsg(storage, socket.tls_id, "Error Loading User.".into(), 1);
    }

    if let Some(client) = storage.server.borrow_mut().clients.get_mut(&socket.tls_id) {
        client.borrow_mut().entity = Some(entity);
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
        return send_infomsg(storage, socket.tls_id, "Error Loading User.".into(), 1);
    };

    send_login_info(world, storage, entity, code, handshake, socket.tls_id, name)
}
