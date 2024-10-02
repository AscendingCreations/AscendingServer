use crate::{
    containers::*, gametypes::*, maps::*, npcs::check_target_distance, players::*, sql::*,
    tasks::*, GlobalKey,
};

//TODO: Add Result<(), AscendingError> to all Functions that return nothing.
pub async fn player_warp(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    new_pos: &Position,
    spawn: bool,
) -> Result<()> {
    if let Some(player) = store.players.get_mut(&key) {
        if player.position.map != new_pos.map {
            //let old_pos = player_switch_maps(world, storage, key, *new_pos).await?;

            /*if !old_pos.1 {
                println!("Failed to switch map");
            }*/

            DataTaskToken::Warp.add_task(map, warp_packet(key, *new_pos)?)?;
            DataTaskToken::Warp.add_task(map, warp_packet(key, *new_pos)?)?;
            DataTaskToken::PlayerSpawn.add_task(map, player_spawn_packet(&player, true)?)?;
            //init_data_lists(world, storage, entity, Some(old_pos.0.map)).await?;
        } else {
            // player_swap_pos(world, storage, entity, *new_pos).await?;

            if spawn {
                DataTaskToken::PlayerSpawn.add_task(map, player_spawn_packet(&player, true)?)?;
                //init_data_lists(world, storage, entity, None).await?;
            } else {
                DataTaskToken::Warp.add_task(map, warp_packet(key, *new_pos)?)?;
            }
        }

        //storage.sql_request.send(SqlRequests::Position(key)).await?;
    }

    Ok(())
}

pub fn get_new_position(store: &mut MapActorStore, info: PlayerInfo, dir: u8) -> PlayerStage {
    if let Some(player) = store.players.get(&info.key) {
        //Down, Right, Up, Left
        let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let mut new_pos = Position::new(
            player.position.x + adj[player.dir as usize].0,
            player.position.y + adj[player.dir as usize].1,
            player.position.map,
        );

        if new_pos.x < 0 || new_pos.x >= 32 || new_pos.y < 0 || new_pos.y >= 32 {
            let adj_pos = [
                (player.position.x, 31),
                (0, player.position.y),
                (player.position.x, 0),
                (31, player.position.y),
            ];
            new_pos = Position::new(
                adj_pos[player.dir as usize].0,
                adj_pos[player.dir as usize].1,
                MapPosition {
                    x: player.position.map.x + adj[player.dir as usize].0,
                    y: player.position.map.y + adj[player.dir as usize].1,
                    group: player.position.map.group,
                },
            );
        }

        PlayerMovementStage::check_blocked(info, (new_pos, dir))
    } else {
        PlayerStage::None(info)
    }
}

pub fn check_blocked(
    map: &mut MapActor,
    store: &mut MapActorStore,
    info: PlayerInfo,
    (next_pos, next_dir): (Position, u8),
) -> PlayerStage {
    if map.map_path_blocked(info.position, next_pos, next_dir, WorldEntityType::Player) {
        //player_warp(map, store, key, &player_position, false).await?;
        return PlayerMovementStage::send_to_original_location(info, next_dir);
    }

    let map_data = map
        .storage
        .bases
        .maps
        .get(&next_pos.map)
        .expect("MapData should exist..");
    let attribute = &map_data.attribute[next_pos.as_tile()];

    if let MapAttribute::Warp(warp_data) = attribute {
        let warp_pos = Position::new(
            warp_data.tile_x as i32,
            warp_data.tile_y as i32,
            MapPosition::new(warp_data.map_x, warp_data.map_y, warp_data.map_group as i32),
        );

        if map.storage.bases.maps.contains_key(&warp_pos.map) {
            //player_warp(map, store, key, &warp_pos, true).await?;
            // we warp instead of moving to next map server side since visually they already appear
            // to move there we just redirect them.
            return PlayerMovementStage::start_player_warp(info, (warp_pos, next_dir));
        }
    }

    if info.position.map != next_pos.map {
        if let std::collections::hash_map::Entry::Vacant(e) =
            store.entity_claims_by_position.entry(next_pos)
        {
            let map_switch_key = store.claims.insert(MapClaims::Tile);

            e.insert(map_switch_key);

            PlayerMovementStage::start_map_switch(info, (next_pos, next_dir), map_switch_key)
        } else {
            PlayerMovementStage::send_to_original_location(info, next_dir)
        }
    } else {
        PlayerMovementStage::move_to_position(info, (next_pos, next_dir))
    }
}

pub fn move_to_position(
    map: &mut MapActor,
    store: &mut MapActorStore,
    info: PlayerInfo,
    (next_pos, next_dir): (Position, u8),
) -> Result<PlayerStage> {
    if let Some(player) = store.players.get_mut(&info.key) {
        let old_pos = player.position;

        map.remove_entity_from_grid(player.position);
        map.add_entity_to_grid(next_pos);
        player.position = next_pos;
        player.dir = next_dir;

        DataTaskToken::Move.add_task(
            map,
            move_packet(
                info.key,
                next_pos,
                old_pos.checkdistance(next_pos) > 1,
                false,
                next_dir,
            )?,
        )?;
    }

    Ok(PlayerStage::None(info))
}

pub async fn player_movement(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    dir: u8,
) -> Result<bool> {
    //Down, Right, Up, Left
    let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let player_position = world.get_or_err::<Position>(entity).await?;
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
        let lock = world.write().await;
        lock.get::<&mut Dir>(entity.0)?.0 = dir;
    }

    if map.map_path_blocked(player_position, new_pos, dir, WorldEntityType::Player) {
        player_warp(map, store, key, &player_position, false).await?;
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
            player_warp(map, store, key, &warp_pos, true).await?;
            return Ok(true);
        }
    }

    storage.sql_request.send(SqlRequests::Position(key)).await?;

    let player_dir = world.get_or_err::<Dir>(entity).await?;
    if new_pos.map != player_position.map {
        let oldpos = player_switch_maps(world, storage, entity, new_pos).await?;

        if !oldpos.1 {
            println!("Failed to switch map");
        }

        DataTaskToken::Move
            .add_task(
                storage,
                move_packet(key, new_pos, false, true, player_dir.0)?,
            )
            .await?;
        DataTaskToken::Move
            .add_task(
                storage,
                move_packet(key, new_pos, false, true, player_dir.0)?,
            )
            .await?;
        DataTaskToken::PlayerSpawn
            .add_task(storage, player_spawn_packet(world, entity, true).await?)
            .await?;

        init_data_lists(world, storage, entity, Some(oldpos.0.map)).await?;
    } else {
        player_swap_pos(world, storage, entity, new_pos).await?;
        DataTaskToken::Move
            .add_task(
                storage,
                move_packet(key, new_pos, false, false, player_dir.0)?,
            )
            .await?;
    }

    Ok(true)
}
