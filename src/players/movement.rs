use chrono::Duration;

use crate::{
    containers::{Entity, EntityKind, GlobalKey, Storage, World},
    gametypes::*,
    maps::*,
    players::*,
    sql::update_pos,
    tasks::*,
};

pub fn player_warp(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    new_pos: &Position,
    spawn: bool,
) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let pos = { p_data.try_lock()?.movement.pos };

        if pos.map != new_pos.map {
            let old_pos = player_switch_maps(world, storage, entity, *new_pos)?;
            if !old_pos.1 {
                println!("Failed to switch map");
            }
            DataTaskToken::Warp(pos.map).add_task(storage, warp_packet(entity, *new_pos)?)?;
            DataTaskToken::Warp(new_pos.map).add_task(storage, warp_packet(entity, *new_pos)?)?;
            DataTaskToken::PlayerSpawn(new_pos.map)
                .add_task(storage, player_spawn_packet(world, entity, true)?)?;
            init_data_lists(world, storage, entity, Some(pos.map))?;
        } else {
            player_swap_pos(world, storage, entity, *new_pos)?;
            if spawn {
                DataTaskToken::PlayerSpawn(new_pos.map)
                    .add_task(storage, player_spawn_packet(world, entity, true)?)?;
                init_data_lists(world, storage, entity, None)?;
            } else {
                DataTaskToken::Warp(new_pos.map)
                    .add_task(storage, warp_packet(entity, *new_pos)?)?;
            }
        }

        let movesavecount = {
            let mut p_data = p_data.try_lock()?;

            p_data.general.movesavecount += 1;

            p_data.general.movesavecount
        };

        if movesavecount >= 25 {
            update_pos(storage, world, entity)?;

            p_data.try_lock()?.general.movesavecount = 0;
        }
    }

    Ok(())
}

pub fn player_movement(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    dir: u8,
) -> Result<bool> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let (player_position, new_pos) = {
            let mut p_data = p_data.try_lock()?;

            //Down, Right, Up, Left
            let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
            let player_position = p_data.movement.pos;

            let mut new_pos = Position::new(
                player_position.x + adj[dir as usize].0,
                player_position.y + adj[dir as usize].1,
                player_position.map,
            );

            if new_pos.x < 0 || new_pos.x >= 32 || new_pos.y < 0 || new_pos.y >= 32 {
                let adj = [
                    (player_position.x, 31),
                    (0, player_position.y),
                    (player_position.x, 0),
                    (31, player_position.y),
                ];
                let map_adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
                new_pos = Position::new(
                    adj[dir as usize].0,
                    adj[dir as usize].1,
                    MapPosition {
                        x: player_position.map.x + map_adj[dir as usize].0,
                        y: player_position.map.y + map_adj[dir as usize].1,
                        group: player_position.map.group,
                    },
                );
            }

            p_data.movement.dir = dir;

            if map_path_blocked(storage, player_position, new_pos, dir, EntityKind::Player) {
                return Ok(false);
            }

            (player_position, new_pos)
        };

        let mapdata = match storage.bases.maps.get(&new_pos.map) {
            Some(data) => data,
            None => return Ok(false),
        };
        let attribute = &mapdata.attribute[new_pos.as_tile()];
        if let MapAttribute::Warp(warpdata) = attribute {
            let warp_pos = Position::new(
                warpdata.tile_x as i32,
                warpdata.tile_y as i32,
                MapPosition::new(warpdata.map_x, warpdata.map_y, warpdata.map_group as i32),
            );
            if storage.bases.maps.contains_key(&warp_pos.map) {
                player_warp(world, storage, entity, &warp_pos, true)?;
                return Ok(true);
            }
        }

        let (player_dir, movesavecount) = {
            let mut p_data = p_data.try_lock()?;

            p_data.general.movesavecount += 1;

            let movesavecount = p_data.general.movesavecount;

            if p_data.general.movesavecount >= 25 {
                p_data.general.movesavecount = 0;
            }

            (p_data.movement.dir, movesavecount)
        };

        if movesavecount >= 25 {
            //update_location(storage, world, entity)?;
        }

        if new_pos.map != player_position.map {
            let oldpos = player_switch_maps(world, storage, entity, new_pos)?;
            if !oldpos.1 {
                println!("Failed to switch map");
            }
            DataTaskToken::Move(oldpos.0.map).add_task(
                storage,
                move_packet(entity, new_pos, false, true, player_dir)?,
            )?;
            DataTaskToken::Move(new_pos.map).add_task(
                storage,
                move_packet(entity, new_pos, false, true, player_dir)?,
            )?;
            DataTaskToken::PlayerSpawn(new_pos.map)
                .add_task(storage, player_spawn_packet(world, entity, true)?)?;

            init_data_lists(world, storage, entity, Some(oldpos.0.map))?;
        } else {
            player_swap_pos(world, storage, entity, new_pos)?;
            DataTaskToken::Move(new_pos.map).add_task(
                storage,
                move_packet(entity, new_pos, false, false, player_dir)?,
            )?;
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn process_player_movement(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let (_old_dir, dir, _socket_id, _old_pos) = {
            let mut p_data = p_data.try_lock()?;

            if p_data.input.stop_move {
                return Ok(());
            }

            let dir = if let Some(dir) = p_data.input.move_dir {
                dir
            } else {
                return Ok(());
            };

            let tick = *storage.gettick.borrow();

            if !p_data.combat.death_type.is_alive()
                || p_data.is_using_type.inuse()
                || p_data.combat.stunned
                || p_data.combat.attacking
                || p_data.movement.move_timer.0 > tick
            {
                return Ok(());
            }

            {
                p_data.movement.move_timer.0 =
                    tick + Duration::try_milliseconds(200).unwrap_or_default();
            }

            (
                p_data.movement.dir,
                dir,
                p_data.socket.id,
                p_data.movement.pos,
            )
        };

        let _ = player_movement(world, storage, entity, dir)?;

        //send_move_ok(storage, socket_id, moveok)?;
    }
    Ok(())
}
