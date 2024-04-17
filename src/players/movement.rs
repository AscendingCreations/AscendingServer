use hecs::World;

use crate::{containers::Storage, gametypes::*, maps::*, players::*, sql::*, tasks::*};

//TODO: Add Result<(), AscendingError> to all Functions that return nothing.
pub fn player_warp(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    new_pos: &Position,
    spawn: bool,
) -> Result<()> {
    if world.get_or_err::<Position>(entity)?.map != new_pos.map {
        let old_pos = player_switch_maps(world, storage, entity, *new_pos)?;
        DataTaskToken::PlayerWarp(old_pos.map)
            .add_task(storage, &WarpPacket::new(*entity, *new_pos))?;
        DataTaskToken::PlayerWarp(new_pos.map)
            .add_task(storage, &WarpPacket::new(*entity, *new_pos))?;
        DataTaskToken::PlayerSpawn(new_pos.map)
            .add_task(storage, &PlayerSpawnPacket::new(world, entity, true)?)?;
        init_data_lists(world, storage, entity, Some(old_pos.map))?;
    } else {
        player_swap_pos(world, storage, entity, *new_pos)?;
        if spawn {
            DataTaskToken::PlayerSpawn(new_pos.map)
                .add_task(storage, &PlayerSpawnPacket::new(world, entity, true)?)?;
            init_data_lists(world, storage, entity, None)?;
        } else {
            DataTaskToken::PlayerWarp(new_pos.map)
                .add_task(storage, &WarpPacket::new(*entity, *new_pos))?;
        }
    }

    {
        world.get::<&mut Player>(entity.0)?.movesavecount += 1;
    }
    if world.get_or_err::<Player>(entity)?.movesavecount >= 25 {
        update_pos(storage, world, entity)?;
        {
            world.get::<&mut Player>(entity.0)?.movesavecount = 0;
        }
    }

    Ok(())
}

pub fn player_movement(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    dir: u8,
) -> Result<bool> {
    //Down, Right, Up, Left
    let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let player_position = world.get_or_err::<Position>(entity)?;
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

    {
        world.get::<&mut Dir>(entity.0)?.0 = dir;
    }

    if !new_pos.update_pos_map(storage) {
        player_warp(world, storage, entity, &player_position, false)?;
        return Ok(false);
    }

    if map_path_blocked(
        storage,
        player_position,
        new_pos,
        dir,
        WorldEntityType::Player,
    ) {
        player_warp(world, storage, entity, &player_position, false)?;
        return Ok(false);
    }

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

    {
        world.get::<&mut Player>(entity.0)?.movesavecount += 1;
    }
    if world.get_or_err::<Player>(entity)?.movesavecount >= 25 {
        update_pos(storage, world, entity)?;
        {
            world.get::<&mut Player>(entity.0)?.movesavecount = 0;
        }
    }

    let player_dir = world.get_or_err::<Dir>(entity)?;
    if new_pos.map != player_position.map {
        let oldpos = player_switch_maps(world, storage, entity, new_pos)?;
        DataTaskToken::PlayerMove(oldpos.map).add_task(
            storage,
            &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
        )?;
        DataTaskToken::PlayerMove(new_pos.map).add_task(
            storage,
            &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
        )?;
        //send_move(world, storage, entity, new_pos, false, true, player_dir.0, Some(oldpos));
        //send_move(world, storage, entity, new_pos, false, true, player_dir.0, None);
        DataTaskToken::PlayerSpawn(new_pos.map)
            .add_task(storage, &PlayerSpawnPacket::new(world, entity, true)?)?;

        init_data_lists(world, storage, entity, Some(oldpos.map))?;
    } else {
        player_swap_pos(world, storage, entity, new_pos)?;
        DataTaskToken::PlayerMove(new_pos.map).add_task(
            storage,
            &MovePacket::new(*entity, new_pos, false, false, player_dir.0),
        )?;
        //send_move(world, storage, entity, new_pos, false, false, player_dir.0, None);
    }

    Ok(true)
}
