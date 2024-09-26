use std::{collections::VecDeque, sync::Arc};

use crate::{gametypes::*, maps::*, npcs::*, tasks::*, time_ext::MyInstant, GlobalKey};
use chrono::Duration;

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

/// This is always called on the local map where the npc originated.
/// If npc was indeed missing this spells trouble.
pub async fn npc_path_start(
    map: &MapActor,
    store: &MapActorStore,
    key: GlobalKey,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store.npcs.get(&key).cloned().expect("NPC Was not on map!");

    let lock = npc.lock().await;
    let position = lock.position;

    if lock.path_timer > map.tick {
        return NpcStage::Movement(MovementStage::ClearTarget {
            key,
            position,
            npc_data,
        });
    }

    let target = lock.target;
    let position = lock.position;

    NpcStage::Movement(MovementStage::GetTargetUpdates {
        key,
        position,
        npc_data,
        target,
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
                let lock = player.lock().await;
                let pos = lock.position;
                let death = lock.death;
                (pos, death)
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
                let lock = npc.lock().await;
                let pos = lock.position;
                let death = lock.death;
                (pos, death)
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
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    new_target: Targeting,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in update_target");
    let mut lock = npc.lock().await;

    if new_target.target_pos.map.group != position.map.group
        || (new_target.target_type == Target::None && lock.target.target_type != Target::None)
    {
        return NpcStage::Movement(MovementStage::ClearTarget {
            key,
            position,
            npc_data,
        });
    }

    //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
    if new_target.target_type != Target::None
        && lock.moving
        && lock.target.target_pos != new_target.target_pos
    {
        lock.target.target_pos = new_target.target_pos;

        if is_next_to_target(map, position, new_target.target_pos, 1) {
            let n_dir = get_target_direction(position, new_target.target_pos);

            if lock.dir != n_dir && lock.set_npc_dir(map, n_dir).is_err() {
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

    if lock.moving && !lock.moves.is_empty() {
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
    let lock = npc.lock().await;

    // Anytime we do A* pathfinding if it failed we will do random movement.
    if let Some(path) = a_star_path(map, position, lock.dir, target_pos) {
        NpcStage::Movement(MovementStage::SetMovePath {
            key,
            position,
            npc_data,
            path,
            timer,
        })
    } else if lock.path_tries + 1 < 10 {
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
    let lock = npc.lock().await;

    let path = npc_rand_movement(map, lock.position);

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
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    dir: u8,
    new_target: Targeting,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in update_ai");
    let mut lock = npc.lock().await;

    // Anytime we do A* pathfinding if it failed we will do random movement.
    if let Some(target_pos) = lock.move_pos_overide {
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

            if dir != n_dir && lock.set_npc_dir(map, n_dir).is_err() {
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
    } else if lock.ai_timer <= map.tick {
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
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    timer: MyInstant,
    path: VecDeque<(Position, u8)>,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    let mut lock = npc.lock().await;

    lock.npc_set_move_path(path);
    lock.reset_path_tries(timer);

    NpcStage::Movement(MovementStage::NextMove {
        key,
        position,
        npc_data,
    })
}

pub async fn clear_move_path(
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    let mut lock = npc.lock().await;

    lock.npc_clear_move_path();
    lock.path_fails = 0;
    lock.path_tries = 0;

    NpcStage::Movement(MovementStage::MoveToCombat {
        key,
        position,
        npc_data,
    })
}

pub async fn clear_target(
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    let mut lock = npc.lock().await;

    lock.npc_clear_move_path();
    lock.target = Targeting::default();
    lock.path_fails = 0;
    lock.path_tries = 0;

    NpcStage::Movement(MovementStage::ProcessMovePosition {
        key,
        position,
        npc_data,
        new_target: Targeting::default(),
    })
}

pub async fn next_move(
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    let mut lock = npc.lock().await;

    let next_move = match lock.moves.pop_front() {
        Some(v) => v,
        None => {
            lock.moving = false;
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
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    let mut lock = npc.lock().await;

    if map.map_path_blocked(position, next_pos, next_dir, WorldEntityType::Npc) {
        if lock.move_pos_overide.is_some() || lock.target.target_type != Target::None {
            if lock.path_fails < 10 {
                //no special movement. Lets wait till we can move again. maybe walkthru upon multi failure here?.
                lock.moves.push_front((next_pos, next_dir));
                lock.path_fails += 1;
            } else {
                lock.npc_clear_move_path();
                lock.path_fails = 0;
                lock.path_tries = 0;
            }
        } else {
            lock.npc_clear_move_path();
            lock.path_fails = 0;
            lock.path_tries = 0;
        }

        return NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        });
    }

    NpcStage::Movement(MovementStage::ProcessMovement {
        key,
        position,
        npc_data,
        next_move: (next_pos, next_dir),
    })
}

pub async fn process_target(
    map: &mut MapActor,
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    target: Targeting,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    //let mut lock = npc.lock().await;

    match target {
        Target::Player(i, _, _) => {
            if let Some(player) = store.players.get(&i) {
                let plock = player.lock().await;

                if plock.death.is_alive() && plock.position == next_pos {
                    return NpcStage::Movement(MovementStage::SetNpcDir {
                        key,
                        position,
                        npc_data,
                        next_move: (next_pos, next_dir),
                    });
                }

                return NpcStage::Movement(MovementStage::ClearMovePath {
                    key,
                    position,
                    npc_data,
                });
            } else {
                return NpcStage::Movement(MovementStage::MoveToCombat {
                    key,
                    position,
                    npc_data,
                });
            }
        }
        Target::Npc(i, _) => {
            if i == key {
                return NpcStage::Movement(MovementStage::MoveToCombat {
                    key,
                    position,
                    npc_data,
                });
            }

            if let Some(npc) = store.npcs.get(&i) {
                let nlock = npc.lock().await;

                if nlock.death.is_alive() && nlock.position == next_pos {
                    return NpcStage::Movement(MovementStage::SetNpcDir {
                        key,
                        position,
                        npc_data,
                        next_move: (next_pos, next_dir),
                    });
                }

                return NpcStage::Movement(MovementStage::ClearMovePath {
                    key,
                    position,
                    npc_data,
                });
            } else {
                return NpcStage::Movement(MovementStage::MoveToCombat {
                    key,
                    position,
                    npc_data,
                });
            }
        }
        _ => {}
    }

    NpcStage::Movement(MovementStage::ProcessMovement {
        key,
        position,
        npc_data,
        next_move: (next_pos, next_dir),
    })
}

pub async fn process_movement(
    map: &mut MapActor,
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    let mut lock = npc.lock().await;

    if lock.position == next_pos {
        lock.set_npc_dir(map, next_dir).unwrap();

        return NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        });
    } else if lock.move_pos_overide.is_none() {
        if lock.target.target_type != Target::None {
            let target = lock.target;

            return NpcStage::Movement(MovementStage::ProcessTarget {
                key,
                position,
                npc_data,
                target,
                next_move: (next_pos, next_dir),
            });
        }
    } else if Some(next_pos) == lock.move_pos_overide {
        lock.move_pos_overide = None;
        lock.npc_clear_move_path();
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
    store: &MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get(&key)
        .expect("Failed to load NPC! in set_move_path");
    let mut lock = npc.lock().await;

    lock.dir = next_dir;
    let old_map = lock.position.map;

    /* if next_pos.map != old_map {
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
            .add_task(storage, npc_spawn_packet(world, entity, true).await?)
            .await?;
    } else {
        npc_swap_pos(world, storage, entity, next.0).await?;
        DataTaskToken::Move(next.0.map)
            .add_task(storage, move_packet(*entity, next.0, false, false, next.1)?)
            .await?;
    }*/

    NpcStage::None
}
