use crate::{gametypes::*, maps::*, npcs::*, tasks::*, time_ext::MyInstant, ClaimsKey};
use chrono::Duration;
use std::collections::VecDeque;

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
pub async fn npc_path_start(map: &MapActor, store: &MapActorStore, npc_info: NpcInfo) -> NpcStage {
    let npc = store
        .npcs
        .get(&npc_info.key)
        .cloned()
        .expect("NPC Was not on map!");

    if npc.path_timer > map.tick {
        return MovementStage::clear_target(npc_info);
    }

    MovementStage::get_target_updates(npc_info, npc.target)
}

pub async fn update_target_pos(
    store: &MapActorStore,
    npc_info: NpcInfo,
    target: Targeting,
) -> NpcStage {
    let (target_pos, death) = match target.target_type {
        Target::Player(global_key, _, _) => {
            if let Some(player) = store.players.get(&global_key) {
                (player.position, player.death)
            } else {
                return MovementStage::clear_target(npc_info);
            }
        }
        Target::Npc(global_key, _) => {
            if let Some(npc) = store.npcs.get(&global_key) {
                (npc.position, npc.death)
            } else {
                return MovementStage::clear_target(npc_info);
            }
        }
        _ => {
            return MovementStage::move_to_combat(npc_info);
        }
    };

    if check_surrounding(npc_info.position.map, target_pos.map, true) == MapPos::None
        || !death.is_alive()
    {
        MovementStage::clear_target(npc_info)
    } else {
        let mut new_target = target;

        new_target.target_pos = target_pos;

        MovementStage::update_target(npc_info, new_target)
    }
}

pub async fn update_target(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    new_target: Targeting,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in update_target");

    if new_target.target_pos.map.group != npc_info.position.map.group
        || (new_target.target_type == Target::None && npc.target.target_type != Target::None)
    {
        return MovementStage::clear_target(npc_info);
    }

    //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
    if new_target.target_type != Target::None
        && npc.moving
        && npc.target.target_pos != new_target.target_pos
    {
        npc.target.target_pos = new_target.target_pos;

        if is_next_to_target(map, npc_info.position, new_target.target_pos, 1) {
            let n_dir = get_target_direction(npc_info.position, new_target.target_pos);

            if npc.dir != n_dir && npc.set_npc_dir(map, n_dir).is_err() {
                return NpcStage::None;
            }
        } else {
            return MovementStage::update_astart_paths(
                npc_info,
                map.tick + Duration::try_milliseconds(100).unwrap_or_default(),
                new_target.target_pos,
            );
        }

        return MovementStage::next_move(npc_info);
    }

    if npc.moving && !npc.moves.is_empty() {
        MovementStage::next_move(npc_info)
    } else {
        MovementStage::process_move_position(npc_info, new_target)
    }
}

pub async fn update_astar_paths(
    map: &mut MapActor,
    store: &MapActorStore,
    npc_info: NpcInfo,
    timer: MyInstant,
    target_pos: Position,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&npc_info.key)
        .expect("Failed to load NPC! in update_target");

    // Anytime we do A* pathfinding if it failed we will do random movement.
    if let Some(path) = a_star_path(map, npc_info.position, npc.dir, target_pos) {
        MovementStage::set_move_path(npc_info, timer, path)
    } else if npc.path_tries + 1 < 10 {
        MovementStage::update_rand_paths(npc_info, timer)
    } else {
        MovementStage::next_move(npc_info)
    }
}

pub async fn update_rand_paths(
    map: &mut MapActor,
    store: &MapActorStore,
    npc_info: NpcInfo,
    timer: MyInstant,
) -> NpcStage {
    let npc = store
        .npcs
        .get(&npc_info.key)
        .expect("Failed to load NPC! in update_target");

    let path = npc_rand_movement(map, npc.position);

    MovementStage::set_move_path(npc_info, timer, path)
}

pub async fn process_moves(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    dir: u8,
    new_target: Targeting,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in update_ai");

    // Anytime we do A* pathfinding if it failed we will do random movement.
    if let Some(target_pos) = npc.move_pos_overide {
        let wait_time = npc_info.data.movement_wait + 750;

        MovementStage::update_astart_paths(
            npc_info,
            map.tick + Duration::try_milliseconds(wait_time).unwrap_or_default(),
            target_pos,
        )
    } else if new_target.target_type != Target::None {
        if is_next_to_target(map, npc_info.position, new_target.target_pos, 1) {
            let n_dir = get_target_direction(npc_info.position, new_target.target_pos);

            if dir != n_dir && npc.set_npc_dir(map, n_dir).is_err() {
                return NpcStage::None;
            }

            MovementStage::next_move(npc_info)
        } else {
            MovementStage::update_astart_paths(
                npc_info,
                map.tick + Duration::try_milliseconds(100).unwrap_or_default(),
                new_target.target_pos,
            )
        }
    } else if npc.ai_timer <= map.tick {
        MovementStage::update_rand_paths(
            npc_info,
            map.tick + Duration::try_milliseconds(3000).unwrap_or_default(),
        )
    } else {
        MovementStage::next_move(npc_info)
    }
}

pub async fn set_move_path(
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    timer: MyInstant,
    path: VecDeque<(Position, u8)>,
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in set_move_path");

    npc.npc_set_move_path(path);
    npc.reset_path_tries(timer);

    MovementStage::next_move(npc_info)
}

pub async fn clear_move_path(store: &mut MapActorStore, npc_info: NpcInfo) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in set_move_path");

    npc.npc_clear_move_path();

    MovementStage::move_to_combat(npc_info)
}

pub async fn clear_target(store: &mut MapActorStore, npc_info: NpcInfo) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in set_move_path");

    npc.npc_clear_move_path();
    npc.target = Targeting::default();

    MovementStage::process_move_position(npc_info, Targeting::default())
}

pub async fn next_move(store: &mut MapActorStore, npc_info: NpcInfo) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in set_move_path");

    let next_move = match npc.moves.pop_front() {
        Some(v) => v,
        None => {
            npc.moving = false;
            return MovementStage::move_to_combat(npc_info);
        }
    };

    MovementStage::check_block(npc_info, next_move)
}

pub async fn check_block(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in set_move_path");

    if map.map_path_blocked(npc_info.position, next_pos, next_dir, WorldEntityType::Npc) {
        if (npc.move_pos_overide.is_some() || npc.target.target_type != Target::None)
            && npc.path_fails < 10
        {
            //no special movement. Lets wait till we can move again. maybe walkthru upon multi failure here?.
            npc.moves.push_front((next_pos, next_dir));
            npc.path_fails += 1;

            return MovementStage::move_to_combat(npc_info);
        }

        npc.npc_clear_move_path();

        return MovementStage::move_to_combat(npc_info);
    }

    // if we have a cliam it means a npc from another map is moving here and already took the spot ahead of time.
    if npc_info.position.map == next_pos.map
        && store.entity_claims_by_position.contains_key(&next_pos)
    {
        npc.npc_clear_move_path();

        MovementStage::move_to_combat(npc_info)
    } else {
        MovementStage::process_movement(npc_info, (next_pos, next_dir))
    }
}

pub async fn process_target(
    store: &MapActorStore,
    npc_info: NpcInfo,
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
            if i == npc_info.key {
                (Death::default(), Position::default(), true)
            } else if let Some(npc) = store.npcs.get(&i) {
                (npc.death, npc.position, false)
            } else {
                (Death::default(), Position::default(), true)
            }
        }
        _ => {
            return MovementStage::process_movement(npc_info, (next_pos, next_dir));
        }
    };

    if go_to_combat {
        MovementStage::move_to_combat(npc_info)
    } else if death.is_alive() && target_pos == next_pos {
        MovementStage::set_npc_dir(npc_info, (next_pos, next_dir))
    } else if death.is_alive() && target_pos != next_pos {
        MovementStage::process_movement(npc_info, (next_pos, next_dir))
    } else {
        MovementStage::clear_move_path(npc_info)
    }
}

pub async fn process_movement(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in set_move_path");

    if npc.position == next_pos {
        npc.set_npc_dir(map, next_dir).unwrap();

        return MovementStage::move_to_combat(npc_info);
    } else if npc.move_pos_overide.is_none() {
        if npc.target.target_type != Target::None {
            let target = npc.target;

            return MovementStage::process_target(npc_info, target, (next_pos, next_dir));
        }
    } else if Some(next_pos) == npc.move_pos_overide {
        npc.move_pos_overide = None;
        npc.npc_clear_move_path();
    }

    MovementStage::finish_move(npc_info, (next_pos, next_dir))
}

pub async fn finish_movement(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    (next_pos, next_dir): (Position, u8),
) -> NpcStage {
    let npc = store
        .npcs
        .get_mut(&npc_info.key)
        .expect("Failed to load NPC! in set_move_path");

    npc.dir = next_dir;

    if next_pos.map != npc.position.map {
        MovementStage::get_tile_claim(npc_info, next_pos)
    } else {
        npc.npc_swap_pos(map, next_pos);
        DataTaskToken::Move
            .add_task(
                map,
                move_packet(npc_info.key, next_pos, false, false, next_dir).unwrap(),
            )
            .unwrap();

        MovementStage::move_to_combat(npc_info)
    }
}

#[inline(always)]
pub async fn check_map_switch(
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    new_position: Position,
) -> NpcStage {
    if let std::collections::hash_map::Entry::Vacant(e) =
        store.entity_claims_by_position.entry(new_position)
    {
        let map_switch_key = store.claims.insert(MapClaims::Tile);

        e.insert(map_switch_key);

        MovementStage::switch_maps(npc_info, new_position, true, map_switch_key)
    } else {
        MovementStage::switch_maps(npc_info, new_position, false, ClaimsKey::default())
    }
}

#[inline(always)]
pub async fn npc_switch_maps(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    new_position: Position,
    map_switch_key: ClaimsKey,
    can_switch: bool,
) -> NpcStage {
    if can_switch {
        let npc = store
            .npcs
            .swap_remove(&npc_info.key)
            .expect("NPC was missing so we could not remove them for map switch.");
        map.remove_entity_from_grid(npc_info.position);

        DataTaskToken::Move
            .add_task(
                map,
                move_packet(npc_info.key, new_position, false, true, npc.dir).unwrap(),
            )
            .unwrap();

        MovementStage::map_switch_finish(npc_info, new_position, map_switch_key, npc)
    } else {
        MovementStage::move_to_combat(npc_info)
    }
}

#[inline(always)]
pub async fn npc_finish_map_switch(
    map: &mut MapActor,
    store: &mut MapActorStore,
    mut npc_info: NpcInfo,
    new_position: Position,
    map_switch_key: ClaimsKey,
    mut npc: Npc,
) -> NpcStage {
    map.add_entity_to_grid(new_position);

    npc.position = new_position;
    DataTaskToken::Move
        .add_task(
            map,
            move_packet(npc_info.key, new_position, false, true, npc.dir).unwrap(),
        )
        .unwrap();
    DataTaskToken::NpcSpawn
        .add_task(map, npc_spawn_packet(&npc, true).unwrap())
        .unwrap();

    store.npcs.insert(npc_info.key, npc);
    store.claims.remove(map_switch_key);
    store.entity_claims_by_position.remove(&new_position);

    npc_info.position = new_position;
    MovementStage::move_to_combat(npc_info)
}
