use crate::{gametypes::*, maps::*, npcs::*, tasks::*, time_ext::MyInstant, ClaimsKey, GlobalKey};
use chrono::Duration;
use std::{collections::VecDeque, sync::Arc};

pub fn is_next_to_target(
    map: &MapActor,
    entity_pos: Position,
    target_pos: Position,
    range: i32,
) -> bool {
    let check = check_surrounding(entity_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    if let Some(dir) = entity_pos.checkdirection(pos) {
        !map.is_dir_blocked(entity_pos, dir as u8) && range >= entity_pos.checkdistance(pos)
    } else {
        false
    }
}

pub fn get_target_direction(entity_pos: Position, target_pos: Position) -> u8 {
    let x_dir = (entity_pos.x - target_pos.x).signum();
    let y_dir = (target_pos.y - entity_pos.y).signum();

    ((x_dir + 2) * x_dir.abs() + (y_dir + 1) * (1 - x_dir.abs())) as u8
}

/// This is always called on the local map where the npc originated.
/// If npc was indeed missing this spells trouble.
pub async fn npc_path_start(
    map: &MapActor,
    store: &MapActorStore,
    key: GlobalKey,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store.npcs.get(&key).cloned().expect("NPC Was not on map!");

    if npc.path_timer > map.tick {
        return NpcStage::Movement(MovementStage::ClearTarget {
            key,
            position: npc.position,
            npc_data,
        });
    }

    NpcStage::Movement(MovementStage::GetTargetUpdates {
        key,
        position: npc.position,
        npc_data,
        target: npc.target,
    })
}

pub async fn update_target_pos(
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    target: Targeting,
) -> NpcStage {
    let (target_pos, death) = match target.target_type {
        Target::Player(global_key, _, _) => {
            if let Some(player) = store.players.get(&global_key) {
                (player.position, player.death)
            } else {
                return NpcStage::Movement(MovementStage::ClearTarget {
                    key,
                    position,
                    npc_data,
                });
            }
        }
        Target::Npc(global_key, _) => {
            if let Some(npc) = store.npcs.get(&global_key) {
                (npc.position, npc.death)
            } else {
                return NpcStage::Movement(MovementStage::ClearTarget {
                    key,
                    position,
                    npc_data,
                });
            }
        }
        _ => {
            return NpcStage::Movement(MovementStage::MoveToCombat {
                key,
                position,
                npc_data,
            })
        }
    };

    if check_surrounding(position.map, target_pos.map, true) == MapPos::None || !death.is_alive() {
        NpcStage::Movement(MovementStage::ClearTarget {
            key,
            position,
            npc_data,
        })
    } else {
        let mut new_target = target;

        new_target.target_pos = target_pos;

        NpcStage::Movement(MovementStage::UpdateTarget {
            key,
            position,
            npc_data,
            new_target,
        })
    }
}

pub async fn update_target(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    new_target: Targeting,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in update_target");

    if new_target.target_pos.map.group != position.map.group
        || (new_target.target_type == Target::None && npc.target.target_type != Target::None)
    {
        return NpcStage::Movement(MovementStage::ClearTarget {
            key,
            position,
            npc_data,
        });
    }

    //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
    if new_target.target_type != Target::None
        && npc.moving
        && npc.target.target_pos != new_target.target_pos
    {
        npc.target.target_pos = new_target.target_pos;

        if is_next_to_target(map, position, new_target.target_pos, 1) {
            let n_dir = get_target_direction(position, new_target.target_pos);

            if npc.dir != n_dir && npc.set_npc_dir(map, n_dir).is_err() {
                return NpcStage::None;
            }
        } else {
            return NpcStage::Movement(MovementStage::UpdateAStarPaths {
                key,
                position,
                npc_data,
                target_pos: new_target.target_pos,
                timer: map.tick + Duration::try_milliseconds(100).unwrap_or_default(),
            });
        }

        return NpcStage::Movement(MovementStage::NextMove {
            key,
            position,
            npc_data,
        });
    }

    if npc.moving && !npc.moves.is_empty() {
        NpcStage::Movement(MovementStage::NextMove {
            key,
            position,
            npc_data,
        })
    } else {
        NpcStage::Movement(MovementStage::ProcessMovePosition {
            key,
            position,
            npc_data,
            new_target,
        })
    }
}

pub async fn update_astar_paths(
    map: &mut MapActor,
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    timer: MyInstant,
    target_pos: Position,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in update_target");

    // Anytime we do A* pathfinding if it failed we will do random movement.
    if let Some(path) = a_star_path(map, position, npc.dir, target_pos) {
        NpcStage::Movement(MovementStage::SetMovePath {
            key,
            position,
            npc_data,
            path,
            timer,
        })
    } else if npc.path_tries + 1 < 10 {
        NpcStage::Movement(MovementStage::UpdateRandPaths {
            key,
            position,
            npc_data,
            timer,
        })
    } else {
        NpcStage::Movement(MovementStage::NextMove {
            key,
            position,
            npc_data,
        })
    }
}

pub async fn update_rand_paths(
    map: &mut MapActor,
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    timer: MyInstant,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in update_target");

    let path = npc_rand_movement(map, npc.position);

    NpcStage::Movement(MovementStage::SetMovePath {
        key,
        position,
        npc_data,
        path,
        timer,
    })
}

pub async fn process_moves(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    dir: u8,
    new_target: Targeting,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in update_ai");

    // Anytime we do A* pathfinding if it failed we will do random movement.
    if let Some(target_pos) = npc.move_pos_overide {
        let wait_time = npc_data.movement_wait + 750;

        return NpcStage::Movement(MovementStage::UpdateAStarPaths {
            key,
            position,
            npc_data,
            target_pos,
            timer: map.tick + Duration::try_milliseconds(wait_time).unwrap_or_default(),
        });
    } else if new_target.target_type != Target::None {
        if is_next_to_target(map, position, new_target.target_pos, 1) {
            let n_dir = get_target_direction(position, new_target.target_pos);

            if dir != n_dir && npc.set_npc_dir(map, n_dir).is_err() {
                return NpcStage::None;
            }

            NpcStage::Movement(MovementStage::NextMove {
                key,
                position,
                npc_data,
            })
        } else {
            NpcStage::Movement(MovementStage::UpdateAStarPaths {
                key,
                position,
                npc_data,
                target_pos: new_target.target_pos,
                timer: map.tick + Duration::try_milliseconds(100).unwrap_or_default(),
            })
        }
    } else if npc.ai_timer <= map.tick {
        NpcStage::Movement(MovementStage::UpdateRandPaths {
            key,
            position,
            npc_data,
            timer: map.tick + Duration::try_milliseconds(3000).unwrap_or_default(),
        })
    } else {
        NpcStage::Movement(MovementStage::NextMove {
            key,
            position,
            npc_data,
        })
    }
}

pub async fn set_move_path(
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    timer: MyInstant,
    path: VecDeque<(Position, u8)>,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in set_move_path");

    npc.npc_set_move_path(path);
    npc.reset_path_tries(timer);

    NpcStage::Movement(MovementStage::NextMove {
        key,
        position,
        npc_data,
    })
}

pub async fn clear_move_path(
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in set_move_path");

    npc.npc_clear_move_path();

    NpcStage::Movement(MovementStage::MoveToCombat {
        key,
        position,
        npc_data,
    })
}

pub async fn clear_target(
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in set_move_path");

    npc.npc_clear_move_path();
    npc.target = Targeting::default();

    NpcStage::Movement(MovementStage::ProcessMovePosition {
        key,
        position,
        npc_data,
        new_target: Targeting::default(),
    })
}

pub async fn next_move(
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in set_move_path");

    let next_move = match npc.moves.pop_front() {
        Some(v) => v,
        None => {
            npc.moving = false;
            return NpcStage::Movement(MovementStage::MoveToCombat {
                key,
                position,
                npc_data,
            });
        }
    };

    NpcStage::Movement(MovementStage::CheckBlock {
        key,
        position,
        npc_data,
        next_move,
    })
}

pub async fn check_block(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in set_move_path");

    if map.map_path_blocked(position, next_pos, next_dir, WorldEntityType::Npc) {
        if (npc.move_pos_overide.is_some() || npc.target.target_type != Target::None)
            && npc.path_fails < 10
        {
            //no special movement. Lets wait till we can move again. maybe walkthru upon multi failure here?.
            npc.moves.push_front((next_pos, next_dir));
            npc.path_fails += 1;

            return NpcStage::Movement(MovementStage::MoveToCombat {
                key,
                position,
                npc_data,
            });
        }

        npc.npc_clear_move_path();

        return NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        });
    }

    // if we have a cliam it means a npc from another map is moving here and already took the spot ahead of time.
    if position.map == next_pos.map && store.entity_claims_by_position.contains_key(&next_pos) {
        npc.npc_clear_move_path();

        NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        })
    } else {
        NpcStage::Movement(MovementStage::ProcessMovement {
            key,
            position,
            npc_data,
            next_move: (next_pos, next_dir),
        })
    }
}

pub async fn process_target(
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    target: Targeting,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let (death, target_pos, go_to_combat) = match target.target_type {
        Target::Player(i, _, _) => {
            if let Some(player) = store.players.get(&i) {
                (player.death, player.position, false)
            } else {
                (Death::default(), Position::default(), true)
            }
        }
        Target::Npc(i, _) => {
            if i == key {
                (Death::default(), Position::default(), true)
            } else if let Some(npc) = store.npcs.get(&i) {
                (npc.death, npc.position, false)
            } else {
                (Death::default(), Position::default(), true)
            }
        }
        _ => {
            return NpcStage::Movement(MovementStage::ProcessMovement {
                key,
                position,
                npc_data,
                next_move: (next_pos, next_dir),
            });
        }
    };

    if go_to_combat {
        NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        })
    } else if death.is_alive() && target_pos == next_pos {
        NpcStage::Movement(MovementStage::SetNpcDir {
            key,
            position,
            npc_data,
            next_move: (next_pos, next_dir),
        })
    } else if death.is_alive() && target_pos != next_pos {
        NpcStage::Movement(MovementStage::ProcessMovement {
            key,
            position,
            npc_data,
            next_move: (next_pos, next_dir),
        })
    } else {
        NpcStage::Movement(MovementStage::ClearMovePath {
            key,
            position,
            npc_data,
        })
    }
}

pub async fn process_movement(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in set_move_path");

    if npc.position == next_pos {
        npc.set_npc_dir(map, next_dir).unwrap();

        return NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        });
    } else if npc.move_pos_overide.is_none() {
        if npc.target.target_type != Target::None {
            let target = npc.target;

            return NpcStage::Movement(MovementStage::ProcessTarget {
                key,
                position,
                npc_data,
                target,
                next_move: (next_pos, next_dir),
            });
        }
    } else if Some(next_pos) == npc.move_pos_overide {
        npc.move_pos_overide = None;
        npc.npc_clear_move_path();
    }

    NpcStage::Movement(MovementStage::FinishMove {
        key,
        position,
        npc_data,
        next_move: (next_pos, next_dir),
    })
}

pub async fn finish_movement(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&key)
        .expect("Failed to load NPC! in set_move_path");

    npc.dir = next_dir;

    if next_pos.map != npc.position.map {
        NpcStage::Movement(MovementStage::GetTileClaim {
            key,
            old_position: position,
            npc_data,
            new_position: next_pos,
        })
    } else {
        npc.npc_swap_pos(map, next_pos);
        DataTaskToken::Move
            .add_task(
                map,
                move_packet(key, next_pos, false, false, next_dir).unwrap(),
            )
            .unwrap();

        NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        })
    }
}

#[inline(always)]
pub async fn check_map_switch(
    store: &mut MapActorStore,
    key: GlobalKey,
    old_position: Position,
    npc_data: Arc<NpcData>,
    new_position: Position,
) -> NpcStage {
    if store.entity_claims_by_position.get(&new_position).is_none() {
        let map_switch_key = store.claims.insert(MapClaims::Tile);

        store
            .entity_claims_by_position
            .insert(new_position, map_switch_key);

        NpcStage::Movement(MovementStage::SwitchMaps {
            key,
            old_position,
            npc_data,
            new_position,
            can_switch: true,
            map_switch_key,
        })
    } else {
        NpcStage::Movement(MovementStage::SwitchMaps {
            key,
            old_position,
            npc_data,
            new_position,
            can_switch: false,
            map_switch_key: ClaimsKey::default(),
        })
    }
}

#[inline(always)]
pub async fn npc_switch_maps(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    old_position: Position,
    npc_data: Arc<NpcData>,
    new_position: Position,
    map_switch_key: ClaimsKey,
    can_switch: bool,
) -> NpcStage {
    if can_switch {
        let npc = store
            .npcs
            .swap_remove(&key)
            .expect("NPC was missing so we could not remove them for map switch.");
        map.remove_entity_from_grid(old_position);

        DataTaskToken::Move
            .add_task(
                map,
                move_packet(key, new_position, false, true, npc.dir).unwrap(),
            )
            .unwrap();

        NpcStage::Movement(MovementStage::MapSwitchFinish {
            key,
            npc_data,
            new_position,
            map_switch_key,
            npc,
        })
    } else {
        NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position: old_position,
            npc_data,
        })
    }
}

#[inline(always)]
pub async fn npc_finish_map_switch(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    npc_data: Arc<NpcData>,
    new_position: Position,
    map_switch_key: ClaimsKey,
    mut npc: Npc,
) -> NpcStage {
    map.add_entity_to_grid(new_position);

    npc.position = new_position;
    DataTaskToken::Move
        .add_task(
            map,
            move_packet(key, new_position, false, true, npc.dir).unwrap(),
        )
        .unwrap();
    DataTaskToken::NpcSpawn
        .add_task(map, npc_spawn_packet(&npc, true).unwrap())
        .unwrap();

    store.npcs.insert(key, npc);
    store.claims.remove(map_switch_key);
    store.entity_claims_by_position.remove(&new_position);

    NpcStage::Movement(MovementStage::MoveToCombat {
        key,
        position: new_position,
        npc_data,
    })
}
