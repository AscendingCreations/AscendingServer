use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, players::Account, tasks::*};
use chrono::Duration;
use hecs::World;

pub fn is_next_to_target(entity_pos: Position, target_pos: Position, range: i32) -> bool {
    let check = check_surrounding(entity_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    if range < entity_pos.checkdistance(pos) {
        return false;
    }

    (target_pos.x == entity_pos.x
        && (target_pos.y >= entity_pos.y - 1 && target_pos.y <= entity_pos.y + 1))
        || (target_pos.y == entity_pos.y
            && (target_pos.x >= entity_pos.x - 1 && target_pos.x <= entity_pos.x + 1))
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

pub fn npc_movement(world: &mut World, storage: &Storage, entity: &Entity, _base: &NpcData) {
    //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
    if world.get_or_panic::<Target>(entity).targettype != EntityType::None
        && storage
            .maps
            .get(&world.get_or_panic::<Position>(entity).map)
            .map(|map| map.borrow().players_on_map())
            .unwrap_or(false)
    {
        let old_pos = world.get_or_panic::<Target>(entity).targetpos;
        update_target_pos(world, entity);

        let pos = world.get_or_panic::<Position>(entity);
        let target_pos = world.get_or_panic::<Target>(entity).targetpos;

        if old_pos != target_pos {
            if is_next_to_target(pos, target_pos, 1) {
                let n_dir = get_target_direction(pos, target_pos);
                if world.get_or_panic::<Dir>(entity).0 != n_dir {
                    set_npc_dir(world, storage, entity, n_dir);
                }
            } else {
                if let Some(path) = a_star_path(
                    storage,
                    pos,
                    world.get_or_panic::<Dir>(entity).0,
                    target_pos,
                ) {
                    npc_set_move_path(world, entity, path);
                }
            }
        }
    }

    if !world.get_or_panic::<NpcMoving>(entity).0
        && world.get::<&NpcMoves>(entity.0).unwrap().0.is_empty()
    {
        if let Some(movepos) = world.get_or_panic::<NpcMovePos>(entity).0 {
            //Move pos overrides targeting pos movement.
            if let Some(path) = a_star_path(
                storage,
                world.get_or_panic::<Position>(entity),
                world.get_or_panic::<Dir>(entity).0,
                movepos,
            ) {
                npc_set_move_path(world, entity, path);
            }
        } else if world.get_or_panic::<Target>(entity).targettype != EntityType::None
            && storage
                .maps
                .get(&world.get_or_panic::<Position>(entity).map)
                .map(|map| map.borrow().players_on_map())
                .unwrap_or(false)
        {
            let pos = world.get_or_panic::<Position>(entity);
            let target_pos = world.get_or_panic::<Target>(entity).targetpos;

            update_target_pos(world, entity);
            if is_next_to_target(pos, target_pos, 1) {
                let n_dir = get_target_direction(pos, target_pos);
                if world.get_or_panic::<Dir>(entity).0 != n_dir {
                    set_npc_dir(world, storage, entity, n_dir);
                }
            } else {
                if let Some(path) = a_star_path(
                    storage,
                    pos,
                    world.get_or_panic::<Dir>(entity).0,
                    target_pos,
                ) {
                    npc_set_move_path(world, entity, path);
                }
            }
        //no special movement lets give them some if we can;
        } else if world.get_or_panic::<NpcAITimer>(entity).0 < *storage.gettick.borrow()
            && storage
                .maps
                .get(&world.get_or_panic::<Position>(entity).map)
                .map(|map| map.borrow().players_on_map())
                .unwrap_or(false)
        {
            let moves = npc_rand_movement(storage, world.get_or_panic::<Position>(entity));
            //get a count of moves to increase the AI wait timer.
            let count = moves.len();

            npc_set_move_path(world, entity, moves);

            world
                .get::<&mut NpcAITimer>(entity.0)
                .expect("Could not find NpcAITimer")
                .0 = *storage.gettick.borrow()
                + Duration::try_milliseconds(count as i64 * 1000).unwrap_or_default();
        }
    }

    if world.get_or_panic::<NpcMoving>(entity).0 {
        let next = match world
            .get::<&mut NpcMoves>(entity.0)
            .expect("Could not find NpcMoves")
            .0
            .pop_front()
        {
            Some(v) => v,
            None => {
                world
                    .get::<&mut NpcMoving>(entity.0)
                    .expect("Could not find NpcMoving")
                    .0 = false;
                return;
            }
        };

        if map_path_blocked(
            storage,
            world.get_or_panic::<Position>(entity),
            next.0,
            next.1,
        ) {
            if world.get_or_panic::<Target>(entity).targettype != EntityType::None
                || world.get_or_panic::<NpcMovePos>(entity).0.is_some()
            {
                world
                    .get::<&mut NpcMoves>(entity.0)
                    .expect("Could not find NpcMoves")
                    .0
                    .push_front(next);
            } else {
                npc_clear_move_path(world, entity);
            }

            return;
        }

        if world.get_or_panic::<Position>(entity) == next.0 {
            set_npc_dir(world, storage, entity, next.1);
        } else {
            if world.get_or_panic::<NpcMovePos>(entity).0.is_none() {
                //do any movepos to position first
                if !storage
                    .maps
                    .get(&world.get_or_panic::<Position>(entity).map)
                    .map(|map| map.borrow().players_on_map())
                    .unwrap_or(false)
                {
                    npc_clear_move_path(world, entity);
                    return;
                }

                match world.get_or_panic::<Target>(entity).targettype {
                    EntityType::Player(i, accid) => {
                        if world.contains(i.0) {
                            if world.get_or_panic::<DeathType>(&i).is_alive()
                                && world.get::<&Account>(i.0).unwrap().id == accid
                            {
                                if world.get_or_panic::<Position>(&i) == next.0 {
                                    npc_clear_move_path(world, entity);
                                    set_npc_dir(world, storage, entity, next.1);
                                    return;
                                }
                            } else {
                                npc_clear_move_path(world, entity);
                            }
                        } else {
                            npc_clear_move_path(world, entity);
                        }
                    }
                    EntityType::Npc(i) => {
                        if world.contains(i.0) {
                            if world.get_or_panic::<DeathType>(&i).is_alive() {
                                if world.get_or_panic::<Position>(&i) == next.0 {
                                    npc_clear_move_path(world, entity);
                                    set_npc_dir(world, storage, entity, next.1);
                                    return;
                                }
                            } else {
                                npc_clear_move_path(world, entity);
                            }
                        } else {
                            npc_clear_move_path(world, entity);
                        }
                    }
                    _ => {}
                };
            } else if Some(next.0) == world.get_or_panic::<NpcMovePos>(entity).0 {
                world
                    .get::<&mut NpcMovePos>(entity.0)
                    .expect("Could not find NpcMovePos")
                    .0 = None;

                npc_clear_move_path(world, entity);
            }

            world
                .get::<&mut Dir>(entity.0)
                .expect("Could not find Dir")
                .0 = next.1;

            let old_map = world.get_or_panic::<Position>(entity).map;
            if next.0.map != old_map {
                npc_switch_maps(world, storage, entity, next.0);
                //Send this Twice one to the old map and one to the new. Just in case people in outermaps did not get it yet.
                let _ = DataTaskToken::NpcMove(old_map).add_task(
                    storage,
                    &MovePacket::new(*entity, next.0, false, true, next.1),
                );
                let _ = DataTaskToken::NpcMove(next.0.map).add_task(
                    storage,
                    &MovePacket::new(*entity, next.0, false, true, next.1),
                );
            } else {
                npc_swap_pos(world, storage, entity, next.0);
                let _ = DataTaskToken::NpcMove(next.0.map).add_task(
                    storage,
                    &MovePacket::new(*entity, next.0, false, false, next.1),
                );
            }
        }
    }
}
