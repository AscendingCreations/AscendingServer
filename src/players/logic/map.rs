use crate::{
    containers::Storage, gametypes::*, maps::*, players::*, sql::*, tasks::*,
};

//TODO: Add Result<(), AscendingError> to all Functions that return nothing.
pub fn player_warp(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &Entity,
    new_pos: &Position,
    spawn: bool,
) {
    if world.get_or_panic::<Position>(entity).map != new_pos.map {
        let old_pos = player_switch_maps(world, storage, entity, *new_pos);
        let _ = DataTaskToken::PlayerWarp(old_pos.map).add_task(
            storage,
            &WarpPacket::new(*entity, *new_pos),
        );
        let _ = DataTaskToken::PlayerWarp(new_pos.map).add_task(
            storage,
            &WarpPacket::new(*entity, *new_pos),
        );
        let _ = DataTaskToken::PlayerSpawn(new_pos.map)
            .add_task(storage, &PlayerSpawnPacket::new(world, entity));
        init_data_lists(world, storage, entity, old_pos.map);
    } else {
        player_swap_pos(world, storage, entity, *new_pos);
        if spawn {
            let _ = DataTaskToken::PlayerSpawn(new_pos.map)
                .add_task(storage, &PlayerSpawnPacket::new(world, entity));
        } else {
            let _ = DataTaskToken::PlayerWarp(new_pos.map).add_task(
                storage,
                &WarpPacket::new(*entity, *new_pos),
            );
        }
    }

    {
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .movesavecount += 1;
    }
    if world.get_or_panic::<Player>(entity).movesavecount >= 25 {
        let _ = update_pos(storage, world, entity);
        {
            world
                .get::<&mut Player>(entity.0)
                .expect("Could not find Player")
                .movesavecount = 0;
        }
    }
}

pub fn player_movement(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &Entity,
    dir: u8,
) -> bool {
    let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let player_position = world.get_or_panic::<Position>(entity);
    let mut new_pos = Position::new(
        player_position.x + adj[dir as usize].0,
        player_position.y + adj[dir as usize].1,
        player_position.map,
    );

    {
        world
            .get::<&mut Dir>(entity.0)
            .expect("Could not find Dir")
            .0 = dir;
    }

    if !new_pos.update_pos_map(storage) {
        player_warp(world, storage, entity, &player_position, false);
        return false;
    }

    if map_path_blocked(storage, player_position, new_pos, dir) {
        player_warp(world, storage, entity, &player_position, false);
        return false;
    }

    //TODO: Process Tile step actions here

    {
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .movesavecount += 1;
    }
    if world.get_or_panic::<Player>(entity).movesavecount >= 25 {
        let _ = update_pos(storage, world, entity);
        {
            world
                .get::<&mut Player>(entity.0)
                .expect("Could not find Player")
                .movesavecount = 0;
        }
    }

    let player_dir = world.get_or_panic::<Dir>(entity);
    if new_pos.map != player_position.map {
        let oldpos = player_switch_maps(world, storage, entity, new_pos);
        let _ = DataTaskToken::PlayerMove(oldpos.map).add_task(
            storage,
            &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
        );
        let _ = DataTaskToken::PlayerMove(new_pos.map).add_task(
            storage,
            &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
        );
        let _ = DataTaskToken::PlayerSpawn(new_pos.map)
            .add_task(storage, &PlayerSpawnPacket::new(world, entity));

        init_data_lists(world, storage, entity, oldpos.map);
    } else {
        player_swap_pos(world, storage, entity, new_pos);
        let _ = DataTaskToken::PlayerMove(player_position.map).add_task(
            storage,
            &MovePacket::new(*entity, player_position, false, false, player_dir.0),
        );
    }

    true
}