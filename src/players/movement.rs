use crate::{
    containers::*, gametypes::*, maps::*, players::*, sql::*, tasks::*, ClaimsKey, GlobalKey,
};
//TODO System to handle incomming packets like move and combat into their own Buffer holder until a
//TODO Currently Active Stage is completed in that Area. This should allow us to seamlessly check and
//TODO Handle combat, target and movement at the same time. Need ability to also know which ones are
//TODO Processing and how to handle them into a buffer when they are.

pub fn get_new_position(store: &mut MapActorStore, info: PlayerInfo, dir: Dir) -> PlayerStage {
    if let Some(player) = store.players.get(&info.key) {
        //Down, Right, Up, Left
        let (x, y) = dir.xy_offset();
        let mut new_pos = Position::new(
            player.position.x + x,
            player.position.y + y,
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
                    x: player.position.map.x + x,
                    y: player.position.map.y + y,
                    group: player.position.map.group,
                },
            );
        }

        PlayerMovementStage::check_blocked(info, (new_pos, dir))
    } else {
        PlayerMovementStage::none(info)
    }
}

pub fn check_blocked(
    map: &mut MapActor,
    store: &mut MapActorStore,
    info: PlayerInfo,
    (next_pos, next_dir): (Position, Dir),
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
    (next_pos, next_dir): (Position, Dir),
) -> Result<PlayerStage> {
    if let Some(player) = store.players.get_mut(&info.key) {
        let old_pos = player.position;

        map.remove_entity_from_grid(player.position);
        map.add_entity_to_grid(next_pos);
        player.position = next_pos;
        player.dir = next_dir;

        //storage.sql_request.send(SqlRequests::Position(key)).await?;
        if old_pos.checkdistance(next_pos) > 1 {
            DataTaskToken::Warp.add_task(map, warp_packet(info.key, next_pos)?)?;
        } else {
            DataTaskToken::Move.add_task(
                map,
                move_packet(info.key, next_pos, false, false, next_dir)?,
            )?;
        }
    }

    Ok(PlayerMovementStage::none(info))
}

pub async fn force_warp(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    next_pos: Position,
) -> Result<()> {
    if next_pos.map == map.position {
        if let Some(player) = store.players.get_mut(&key) {
            map.remove_entity_from_grid(player.position);
            map.add_entity_to_grid(next_pos);
            player.position = next_pos;

            //storage.sql_request.send(SqlRequests::Position(key)).await?;
            DataTaskToken::Warp.add_task(map, warp_packet(key, next_pos)?)?;
        }
    } else if let Some(player) = store.players.swap_remove(&key) {
        map.remove_entity_from_grid(player.position);

        DataTaskToken::Warp.add_task(map, warp_packet(key, next_pos)?)?;
        PlayerMovementStage::finish_player_warp(
            PlayerInfo::new(key, player.position),
            (next_pos, player.dir),
            player,
        )
        .send(map)
        .await
    }

    Ok(())
}

pub fn start_map_switch(
    map: &mut MapActor,
    store: &mut MapActorStore,
    info: PlayerInfo,
    (next_pos, next_dir): (Position, Dir),
    claim: ClaimsKey,
) -> Result<PlayerStage> {
    let Some(player) = store.players.swap_remove(&info.key) else {
        return Ok(PlayerMovementStage::none(info));
    };

    map.remove_entity_from_grid(player.position);

    DataTaskToken::Move.add_task(map, move_packet(info.key, next_pos, false, true, next_dir)?)?;

    Ok(PlayerMovementStage::finish_map_switch(
        info,
        (next_pos, next_dir),
        claim,
        player,
    ))
}

pub fn start_map_warp(
    map: &mut MapActor,
    store: &mut MapActorStore,
    info: PlayerInfo,
    (next_pos, next_dir): (Position, Dir),
) -> Result<PlayerStage> {
    if next_pos.map == map.position {
        Ok(PlayerMovementStage::move_to_position(
            info,
            (next_pos, next_dir),
        ))
    } else {
        let Some(player) = store.players.swap_remove(&info.key) else {
            return Ok(PlayerMovementStage::none(info));
        };

        map.remove_entity_from_grid(player.position);

        DataTaskToken::Warp.add_task(map, warp_packet(info.key, next_pos)?)?;

        // when warping a player can always Go to the tile regardless of blocked or not.
        Ok(PlayerMovementStage::finish_player_warp(
            info,
            (next_pos, next_dir),
            player,
        ))
    }
}

pub fn finish_map_switch(
    map: &mut MapActor,
    store: &mut MapActorStore,
    info: PlayerInfo,
    (next_pos, next_dir): (Position, Dir),
    claim: ClaimsKey,
    mut player: Player,
) -> Result<PlayerStage> {
    player.position = next_pos;
    player.dir = next_dir;

    map.add_entity_to_grid(player.position);

    //storage.sql_request.send(SqlRequests::Position(key)).await?;
    DataTaskToken::Move.add_task(map, move_packet(info.key, next_pos, false, true, next_dir)?)?;
    DataTaskToken::PlayerSpawn.add_task(map, player_spawn_packet(&player, true)?)?;

    //TODO Ask New maps to Send you New entities infomation.

    store.players.insert(info.key, player);
    store.claims.remove(claim);
    store.entity_claims_by_position.remove(&next_pos);

    Ok(PlayerMovementStage::none(info))
}

pub fn finish_map_warp(
    map: &mut MapActor,
    store: &mut MapActorStore,
    info: PlayerInfo,
    (next_pos, next_dir): (Position, Dir),
    mut player: Player,
) -> Result<PlayerStage> {
    player.position = next_pos;
    player.dir = next_dir;

    //storage.sql_request.send(SqlRequests::Position(key)).await?;
    map.add_entity_to_grid(player.position);

    DataTaskToken::Warp.add_task(map, warp_packet(info.key, next_pos)?)?;
    DataTaskToken::Dir.add_task(map, dir_packet(info.key, next_dir)?)?;
    DataTaskToken::PlayerSpawn.add_task(map, player_spawn_packet(&player, true)?)?;

    //TODO Ask New maps to Send you New entities infomation.
    store.players.insert(info.key, player);

    Ok(PlayerMovementStage::none(info))
}
