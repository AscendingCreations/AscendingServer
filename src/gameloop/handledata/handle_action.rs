use chrono::Duration;
use mmap_bytey::MByteBuffer;

use crate::{
    containers::{Entity, GlobalKey, Storage, World},
    gametypes::*,
    maps::player_interact_object,
    players::{player_combat, player_movement, player_warp},
    socket::send_move_ok,
    tasks::{DataTaskToken, dir_packet},
};

use super::SocketID;

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
