use crate::{containers::{GameStore, GameWorld}, gametypes::*, maps::*, npcs::*, tasks::*};
use chrono::Duration;
use hecs::World;

pub async fn is_next_to_target(
    storage: &GameStore,
    entity_pos: Position,
    target_pos: Position,
    range: i32,
) -> bool {
    let check = check_surrounding(entity_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    if let Some(dir) = entity_pos.checkdirection(pos) {
        !is_dir_blocked(storage, entity_pos, dir as u8).await
            && range >= entity_pos.checkdistance(pos)
    } else {
        false
    }
}

pub fn get_target_direction(entity_pos: Position, target_pos: Position) -> u8 {
    if entity_pos.x > target_pos.x {
        3
    } else if entity_pos.x < target_pos.x {
        1
    } else if entity_pos.y < target_pos.y {
        2
    } else {
        0
    }
}

pub async fn npc_update_path(
    world: &mut World,
    storage: &GameStore,
    entity: &Entity,
    base: &NpcData,
) -> Result<()> {
    let path_timer = world.get_or_err::<NpcPathTimer>(entity)?;

    if path_timer.timer > *storage.gettick.lock().await {
        return Ok(());
    }

    let npc_moving = world.get_or_err::<NpcMoving>(entity)?.0;
    let target = world.get_or_err::<Target>(entity)?;
    let position = world.get_or_err::<Position>(entity)?;
    let players_on_map = check_players_on_map(world, storage, &position.map).await;
    let mut new_target = target;

    if target.target_type != EntityType::None {
        new_target = update_target_pos(world, entity).await?;
    }

    if new_target.target_pos.map.group != position.map.group
        || (new_target.target_type == EntityType::None && target.target_type != EntityType::None)
    {
        {
            let mut path_tmr = world.get::<&mut NpcPathTimer>(entity.0)?;
            path_tmr.tries = 0;
            path_tmr.fails = 0;
        }
        *world.get::<&mut Target>(entity.0)? = Target::default();
        new_target = Target::default();
        npc_clear_move_path(world, entity).await?;
    }

    //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
    if new_target.target_type != EntityType::None
        && players_on_map
        && npc_moving
        && target.target_pos != new_target.target_pos
    {
        if is_next_to_target(storage, position, new_target.target_pos, 1).await {
            let n_dir = get_target_direction(position, new_target.target_pos);
            if world.get_or_err::<Dir>(entity)?.0 != n_dir {
                set_npc_dir(world, storage, entity, n_dir).await?;
            }
        } else if let Some(path) = a_star_path(
            storage,
            position,
            world.get_or_err::<Dir>(entity)?.0,
            new_target.target_pos,
        )
        .await
        {
            npc_set_move_path(world, entity, path).await?;
            {
                let mut path_tmr = world.get::<&mut NpcPathTimer>(entity.0)?;
                path_tmr.tries = 0;
                path_tmr.timer = *storage.gettick.lock().await
                    + Duration::try_milliseconds(100).unwrap_or_default();
                path_tmr.fails = 0;
            }
        }

        return Ok(());
    }

    if npc_moving && !world.get::<&NpcMoves>(entity.0)?.0.is_empty() {
        return Ok(());
    }

    if let Some(movepos) = world.get_or_err::<NpcMovePos>(entity)?.0 {
        //Move pos overrides targeting pos movement.
        if let Some(path) = a_star_path(
            storage,
            world.get_or_err::<Position>(entity)?,
            world.get_or_err::<Dir>(entity)?.0,
            movepos,
        )
        .await
        {
            npc_set_move_path(world, entity, path).await?;
        }

        {
            let mut path_tmr = world.get::<&mut NpcPathTimer>(entity.0)?;
            path_tmr.tries = 0;
            path_tmr.timer = *storage.gettick.lock().await
                + Duration::try_milliseconds(base.movement_wait + 750).unwrap_or_default();
            path_tmr.fails = 0;
        }
    } else if new_target.target_type != EntityType::None && players_on_map {
        if is_next_to_target(storage, position, new_target.target_pos, 1).await {
            let n_dir = get_target_direction(position, new_target.target_pos);
            if world.get_or_err::<Dir>(entity)?.0 != n_dir {
                set_npc_dir(world, storage, entity, n_dir).await?;
            }
        } else if let Some(path) = a_star_path(
            storage,
            position,
            world.get_or_err::<Dir>(entity)?.0,
            new_target.target_pos,
        )
        .await
        {
            npc_set_move_path(world, entity, path).await?;
            {
                let mut path_tmr = world.get::<&mut NpcPathTimer>(entity.0)?;
                path_tmr.tries = 0;
                path_tmr.timer = *storage.gettick.lock().await
                    + Duration::try_milliseconds(100).unwrap_or_default();
                path_tmr.fails = 0;
            }
        } else if path_timer.tries + 1 < 10 {
            let moves = npc_rand_movement(storage, world.get_or_err::<Position>(entity)?).await;
            npc_set_move_path(world, entity, moves).await?;

            {
                let mut path_tmr = world.get::<&mut NpcPathTimer>(entity.0)?;
                path_tmr.tries += 1;
                path_tmr.timer = *storage.gettick.lock().await
                    + Duration::try_milliseconds(
                        base.movement_wait + ((path_timer.tries + 1) as i64 * 250),
                    )
                    .unwrap_or_default();
                world.get::<&mut NpcAITimer>(entity.0)?.0 = *storage.gettick.lock().await
                    + Duration::try_milliseconds(3000).unwrap_or_default();
            }
        } else {
            {
                let mut path_tmr = world.get::<&mut NpcPathTimer>(entity.0)?;
                path_tmr.tries = 0;
                path_tmr.fails = 0;
            }
            *world.get::<&mut Target>(entity.0)? = Target::default();
            npc_clear_move_path(world, entity).await?;
        }
    //no special movement lets give them some if we can;
    } else if world.get_or_err::<NpcAITimer>(entity)?.0 <= *storage.gettick.lock().await
        && check_players_on_map(world, storage, &world.get_or_err::<Position>(entity)?.map).await
    {
        let moves = npc_rand_movement(storage, world.get_or_err::<Position>(entity)?).await;

        npc_set_move_path(world, entity, moves).await?;

        world.get::<&mut NpcAITimer>(entity.0)?.0 =
            *storage.gettick.lock().await + Duration::try_milliseconds(3000).unwrap_or_default();
    }

    Ok(())
}

pub async fn check_players_on_map(
    _world: &mut World,
    storage: &GameStore,
    position: &MapPosition,
) -> bool {
    if let Some(map) = storage.maps.get(position) {
        map.lock().await.players_on_map()
    } else {
        false
    }
}

pub async fn npc_movement(
    world: &mut World,
    storage: &GameStore,
    entity: &Entity,
    _base: &NpcData,
) -> Result<()> {
    if world.get_or_err::<NpcMoving>(entity)?.0 {
        let position = world.get_or_err::<Position>(entity)?;

        let next = match world.get::<&mut NpcMoves>(entity.0)?.0.pop_front() {
            Some(v) => v,
            None => {
                world.get::<&mut NpcMoving>(entity.0)?.0 = false;
                return Ok(());
            }
        };

        if map_path_blocked(storage, position, next.0, next.1, WorldEntityType::Npc).await {
            if world.get_or_err::<NpcMovePos>(entity)?.0.is_some()
                || world.get_or_err::<Target>(entity)?.target_type != EntityType::None
            {
                if world.get::<&NpcPathTimer>(entity.0)?.fails < 10 {
                    //no special movement. Lets wait till we can move again. maybe walkthru upon multi failure here?.
                    world.get::<&mut NpcMoves>(entity.0)?.0.push_front(next);
                    world.get::<&mut NpcPathTimer>(entity.0)?.fails += 1;
                } else {
                    {
                        let mut path_tmr = world.get::<&mut NpcPathTimer>(entity.0)?;
                        path_tmr.tries = 0;
                        path_tmr.fails = 0;
                    }

                    npc_clear_move_path(world, entity).await?;
                }
            } else {
                npc_clear_move_path(world, entity).await?;
            }

            return Ok(());
        }

        if position == next.0 {
            set_npc_dir(world, storage, entity, next.1).await?;
        } else {
            if world.get_or_err::<NpcMovePos>(entity)?.0.is_none() {
                //do any movepos to position first
                if !check_players_on_map(world, storage, &position.map).await {
                    npc_clear_move_path(world, entity).await?;
                    return Ok(());
                }

                match world.get_or_err::<Target>(entity)?.target_type {
                    EntityType::Player(i, _) => {
                        if world.contains(i.0) {
                            if world.get_or_err::<DeathType>(&i)?.is_alive()
                                && world.get_or_err::<Position>(&i)? == next.0
                            {
                                npc_clear_move_path(world, entity).await?;
                                set_npc_dir(world, storage, entity, next.1).await?;
                                return Ok(());
                            } else {
                                npc_clear_move_path(world, entity).await?;
                            }
                        } else {
                            npc_clear_move_path(world, entity).await?;
                        }
                    }
                    EntityType::Npc(i) => {
                        if world.contains(i.0) {
                            if world.get_or_err::<DeathType>(&i)?.is_alive()
                                && world.get_or_err::<Position>(&i)? == next.0
                            {
                                npc_clear_move_path(world, entity).await?;
                                set_npc_dir(world, storage, entity, next.1).await?;
                                return Ok(());
                            } else {
                                npc_clear_move_path(world, entity).await?;
                            }
                        } else {
                            npc_clear_move_path(world, entity).await?;
                        }
                    }
                    _ => {}
                };
            } else if Some(next.0) == world.get_or_err::<NpcMovePos>(entity)?.0 {
                world.get::<&mut NpcMovePos>(entity.0)?.0 = None;

                npc_clear_move_path(world, entity).await?;
            }

            world.get::<&mut Dir>(entity.0)?.0 = next.1;

            let old_map = position.map;
            if next.0.map != old_map {
                npc_switch_maps(world, storage, entity, next.0).await?;
                //Send this Twice one to the old map and one to the new. Just in case people in outermaps did not get it yet.
                DataTaskToken::Move(old_map)
                    .add_task(storage, move_packet(*entity, next.0, false, true, next.1)?)
                    .await?;
                //TODO Test this to see if we need this or if we do to migrate it to Spawn instead.
                DataTaskToken::Move(next.0.map)
                    .add_task(storage, move_packet(*entity, next.0, false, true, next.1)?)
                    .await?;
                DataTaskToken::NpcSpawn(next.0.map)
                    .add_task(storage, npc_spawn_packet(world, entity, true)?)
                    .await?;
            } else {
                npc_swap_pos(world, storage, entity, next.0).await?;
                DataTaskToken::Move(next.0.map)
                    .add_task(storage, move_packet(*entity, next.0, false, false, next.1)?)
                    .await?;
            }
        }
    }

    Ok(())
}
