use crate::{
    containers::{Entity, EntityKind, GlobalKey, Storage, Target, World},
    gametypes::*,
    maps::*,
    npcs::*,
    tasks::*,
};
use chrono::Duration;

pub fn is_next_to_target(
    storage: &Storage,
    entity_pos: Position,
    target_pos: Position,
    range: i32,
) -> bool {
    let check = check_surrounding(entity_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    if let Some(dir) = entity_pos.checkdirection(pos) {
        !is_dir_blocked(storage, entity_pos, dir as u8) && range >= entity_pos.checkdistance(pos)
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

pub fn npc_update_path(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    base: &NpcData,
) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let (npc_moving, target, position, dir, move_pos, path_timer) = {
            let n_data = n_data.try_lock()?;

            if n_data.path_timer.timer > *storage.gettick.borrow() {
                return Ok(());
            }

            (
                n_data.moving,
                n_data.combat.target,
                n_data.movement.pos,
                n_data.movement.dir,
                n_data.move_pos,
                n_data.path_timer,
            )
        };

        let players_on_map = storage
            .maps
            .get(&position.map)
            .map(|map| map.borrow().players_on_map())
            .unwrap_or(false);
        let mut new_target = target;

        if target.target_entity.is_some() {
            new_target = update_target_pos(world, entity)?;
        }

        if new_target.target_pos.map.group != position.map.group
            || (new_target.target_entity.is_none() && target.target_entity.is_some())
        {
            {
                let mut n_data = n_data.try_lock()?;

                n_data.path_timer.tries = 0;
                n_data.path_timer.fails = 0;

                n_data.combat.target = Target::default();
            }

            new_target = Target::default();
            npc_clear_move_path(world, entity)?;
        }

        //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
        if new_target.target_entity.is_some()
            && players_on_map
            && npc_moving
            && target.target_pos != new_target.target_pos
        {
            if is_next_to_target(storage, position, new_target.target_pos, 1) {
                let n_dir = get_target_direction(position, new_target.target_pos);
                if dir != n_dir {
                    set_npc_dir(world, storage, entity, n_dir)?;
                }
            } else if let Some(path) = a_star_path(storage, position, dir, new_target.target_pos) {
                npc_set_move_path(world, entity, path)?;
                {
                    let mut n_data = n_data.try_lock()?;

                    n_data.path_timer.tries = 0;
                    n_data.path_timer.timer = *storage.gettick.borrow()
                        + Duration::try_milliseconds(100).unwrap_or_default();
                    n_data.path_timer.fails = 0;
                }
            }

            return Ok(());
        }

        if npc_moving && !{ n_data.try_lock()?.moves.0.is_empty() } {
            return Ok(());
        }

        if let Some(movepos) = move_pos.0 {
            //Move pos overrides targeting pos movement.
            if let Some(path) = a_star_path(storage, position, dir, movepos) {
                npc_set_move_path(world, entity, path)?;
            }

            {
                let mut n_data = n_data.try_lock()?;

                n_data.path_timer.tries = 0;
                n_data.path_timer.timer = *storage.gettick.borrow()
                    + Duration::try_milliseconds(base.movement_wait + 750).unwrap_or_default();
                n_data.path_timer.fails = 0;
            }
        } else if new_target.target_entity.is_some() && players_on_map {
            if is_next_to_target(storage, position, new_target.target_pos, 1) {
                let n_dir = get_target_direction(position, new_target.target_pos);
                if dir != n_dir {
                    set_npc_dir(world, storage, entity, n_dir)?;
                }
            } else if let Some(path) = a_star_path(storage, position, dir, new_target.target_pos) {
                npc_set_move_path(world, entity, path)?;
                {
                    let mut n_data = n_data.try_lock()?;

                    n_data.path_timer.tries = 0;
                    n_data.path_timer.timer = *storage.gettick.borrow()
                        + Duration::try_milliseconds(100).unwrap_or_default();
                    n_data.path_timer.fails = 0;
                }
            } else if path_timer.tries + 1 < 10 {
                let moves = npc_rand_movement(storage, position);
                npc_set_move_path(world, entity, moves)?;

                {
                    let mut n_data = n_data.try_lock()?;

                    n_data.path_timer.tries += 1;
                    n_data.path_timer.timer = *storage.gettick.borrow()
                        + Duration::try_milliseconds(
                            base.movement_wait + ((path_timer.tries + 1) as i64 * 250),
                        )
                        .unwrap_or_default();
                    n_data.ai_timer.0 = *storage.gettick.borrow()
                        + Duration::try_milliseconds(3000).unwrap_or_default();
                }
            } else {
                {
                    let mut n_data = n_data.try_lock()?;

                    n_data.path_timer.tries = 0;
                    n_data.path_timer.fails = 0;

                    n_data.combat.target = Target::default();
                }

                npc_clear_move_path(world, entity)?;
            }
        //no special movement lets give them some if we can;
        } else if { n_data.try_lock()?.ai_timer.0 <= *storage.gettick.borrow() }
            && storage
                .maps
                .get(&position.map)
                .map(|map| map.borrow().players_on_map())
                .unwrap_or(false)
        {
            let moves = npc_rand_movement(storage, position);

            npc_set_move_path(world, entity, moves)?;

            n_data.try_lock()?.ai_timer.0 =
                *storage.gettick.borrow() + Duration::try_milliseconds(3000).unwrap_or_default();
        }
    }
    Ok(())
}

pub fn npc_movement(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    _base: &NpcData,
) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let (position, next, target, path_timer, move_pos) = {
            let mut n_data = n_data.try_lock()?;

            if !n_data.moving {
                return Ok(());
            }

            (
                n_data.movement.pos,
                match n_data.moves.0.pop_front() {
                    Some(v) => v,
                    None => {
                        n_data.moving = false;
                        return Ok(());
                    }
                },
                n_data.combat.target,
                n_data.path_timer,
                n_data.move_pos.0,
            )
        };

        if map_path_blocked(storage, position, next.0, next.1, EntityKind::Npc) {
            if move_pos.is_some() || target.target_entity.is_some() {
                if path_timer.fails < 10 {
                    let mut n_data = n_data.try_lock()?;

                    //no special movement. Lets wait till we can move again. maybe walkthru upon multi failure here?.
                    n_data.moves.0.push_front(next);
                    n_data.path_timer.fails += 1;
                } else {
                    {
                        let mut n_data = n_data.try_lock()?;

                        n_data.path_timer.tries = 0;
                        n_data.path_timer.fails = 0;
                    }

                    npc_clear_move_path(world, entity)?;
                }
            } else {
                npc_clear_move_path(world, entity)?;
            }

            return Ok(());
        }

        if position == next.0 {
            set_npc_dir(world, storage, entity, next.1)?;
        } else {
            if move_pos.is_none() {
                //do any movepos to position first
                if !storage
                    .maps
                    .get(&position.map)
                    .map(|map| map.borrow().players_on_map())
                    .unwrap_or(false)
                {
                    npc_clear_move_path(world, entity)?;
                    return Ok(());
                }

                if let Some(target_entity) = target.target_entity {
                    match world.get_entity(target_entity) {
                        Ok(result_data) => match result_data {
                            Entity::Player(p_data) => {
                                let (t_pos, t_death_type) = {
                                    let p_data = p_data.try_lock()?;
                                    (p_data.movement.pos, p_data.combat.death_type)
                                };

                                if t_death_type.is_alive() {
                                    if t_pos == next.0 {
                                        npc_clear_move_path(world, entity)?;
                                        set_npc_dir(world, storage, entity, next.1)?;
                                        return Ok(());
                                    }
                                } else {
                                    npc_clear_move_path(world, entity)?;
                                }
                            }
                            Entity::Npc(n2_data) => {
                                let (t_pos, t_death_type) = {
                                    let n2_data = n2_data.try_lock()?;
                                    (n2_data.movement.pos, n2_data.combat.death_type)
                                };

                                if t_death_type.is_alive() {
                                    if t_pos == next.0 {
                                        npc_clear_move_path(world, entity)?;
                                        set_npc_dir(world, storage, entity, next.1)?;
                                        return Ok(());
                                    }
                                } else {
                                    npc_clear_move_path(world, entity)?;
                                }
                            }
                            _ => {}
                        },
                        Err(_) => {
                            npc_clear_move_path(world, entity)?;
                        }
                    }
                }
            } else if Some(next.0) == move_pos {
                {
                    n_data.try_lock()?.move_pos.0 = None;
                }

                npc_clear_move_path(world, entity)?;
            }

            {
                n_data.try_lock()?.movement.dir = next.1;
            }

            let old_map = position.map;
            if next.0.map != old_map {
                npc_switch_maps(world, storage, entity, next.0)?;
                //Send this Twice one to the old map and one to the new. Just in case people in outermaps did not get it yet.
                DataTaskToken::Move(old_map)
                    .add_task(storage, move_packet(entity, next.0, false, true, next.1)?)?;
                //TODO Test this to see if we need this or if we do to migrate it to Spawn instead.
                DataTaskToken::Move(next.0.map)
                    .add_task(storage, move_packet(entity, next.0, false, true, next.1)?)?;
                DataTaskToken::NpcSpawn(next.0.map)
                    .add_task(storage, npc_spawn_packet(world, entity, true)?)?;
            } else {
                npc_swap_pos(world, storage, entity, next.0)?;
                DataTaskToken::Move(next.0.map)
                    .add_task(storage, move_packet(entity, next.0, false, false, next.1)?)?;
            }
        }
    }

    Ok(())
}
