use crate::{containers::*, gametypes::*, maps::*, players::*, sql::*, tasks::*, GlobalKey};

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

        DataTaskToken::Move(oldpos.0.map)
            .add_task(
                storage,
                move_packet(key, new_pos, false, true, player_dir.0)?,
            )
            .await?;
        DataTaskToken::Move(new_pos.map)
            .add_task(
                storage,
                move_packet(key, new_pos, false, true, player_dir.0)?,
            )
            .await?;
        DataTaskToken::PlayerSpawn(new_pos.map)
            .add_task(storage, player_spawn_packet(world, entity, true).await?)
            .await?;

        init_data_lists(world, storage, entity, Some(oldpos.0.map)).await?;
    } else {
        player_swap_pos(world, storage, entity, new_pos).await?;
        DataTaskToken::Move(new_pos.map)
            .add_task(
                storage,
                move_packet(key, new_pos, false, false, player_dir.0)?,
            )
            .await?;
    }

    Ok(true)
}
