use crate::{gametypes::*, maps::*, npcs::*, GlobalKey};
use chrono::Duration;

pub fn check_target(store: &mut MapActorStore, npc_info: NpcInfo, target: Targeting) -> NpcStage {
    let mut is_valid = false;

    match target.target {
        Target::Player {
            key,
            uid,
            position: _,
        } => {
            if let Some(player) = store.players.get(&key) {
                if player.death.is_alive() && player.uid == uid {
                    is_valid = true;
                }
            }
        }
        Target::Npc { key, position: _ } => {
            if let Some(npc) = store.npcs.get(&key) {
                if npc.death.is_alive() {
                    is_valid = true;
                }
            }
        }
        _ => {}
    }

    if is_valid {
        TargetingStage::detarget_chance(npc_info, target)
    } else {
        TargetingStage::clear_target(npc_info)
    }
}

pub fn check_target_distance(
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    target: Targeting,
) -> NpcStage {
    let entity_pos = match target.target {
        Target::Player {
            key,
            uid: _,
            position: _,
        } => store.players.get(&key).map(|player| player.position),
        Target::Npc { key, position: _ } => store.npcs.get(&key).map(|npc| npc.position),
        Target::Map(position) => Some(position),
        _ => None,
    };

    if let Some(entity_pos) = entity_pos {
        if npc_info.position.checkdistance(entity_pos) > npc_info.data.sight {
            return TargetingStage::clear_target(npc_info);
        }
    }

    TargetingStage::move_to_movement(npc_info)
}

pub fn clear_target(store: &mut MapActorStore, npc_info: NpcInfo) -> NpcStage {
    if let Some(npc) = store.npcs.get_mut(&npc_info.key) {
        npc.target = Targeting::default();
    }

    TargetingStage::move_to_movement(npc_info)
}

pub fn check_detargeting(map: &mut MapActor, npc_info: NpcInfo, target: Targeting) -> NpcStage {
    if target.target != Target::None {
        if npc_info.data.target_auto_switch && target.timer < map.tick {
            return TargetingStage::clear_target(npc_info);
        } else if npc_info.data.target_range_dropout {
            return NpcStage::Targeting(TargetingStage::CheckDistance { npc_info, target });
        }

        return TargetingStage::move_to_movement(npc_info);
    }

    NpcStage::Targeting(TargetingStage::GetTargetMaps { npc_info })
}

pub fn get_targeting_maps(map: &mut MapActor, npc_info: NpcInfo) -> NpcStage {
    if !npc_info.data.is_agressive() {
        return TargetingStage::move_to_movement(npc_info);
    }

    let maps = get_maps_in_range(&map.storage, &npc_info.position, npc_info.data.sight)
        .iter()
        .filter_map(|m| m.get())
        .collect();

    TargetingStage::get_target_from_maps(npc_info, maps)
}

pub fn get_target(store: &mut MapActorStore, npc_info: NpcInfo) -> NpcStage {
    for (pkey, player) in store.players.iter_mut() {
        if let Some(Entity::Player(player)) =
            npc_targeting(npc_info.position, &npc_info.data, Entity::Player(player))
        {
            let target = Target::player(*pkey, player.uid, player.position);

            return TargetingStage::set_target(npc_info, target, player.position);
        }
    }

    if npc_info.data.has_enemies {
        for (nkey, npc) in store.npcs.iter_mut() {
            if npc_info.key == *nkey {
                continue;
            }

            if let Some(Entity::Npc(npc)) =
                npc_targeting(npc_info.position, &npc_info.data, Entity::Npc(npc))
            {
                let target = Target::npc(*nkey, npc.position);

                return TargetingStage::set_target(npc_info, target, npc.position);
            }
        }
    }

    //Tell system we found nothing to do next or something else like movement.
    TargetingStage::move_to_movement(npc_info)
}

pub fn set_target(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    target: Target,
    target_pos: Position,
) -> NpcStage {
    if let Some(npc) = store.npcs.get_mut(&npc_info.key) {
        npc.target.target = target;
        npc.target.timer = map.tick
            + Duration::try_milliseconds(npc_info.data.target_auto_switch_chance)
                .unwrap_or_default();
        npc.target.update_pos(target_pos);
        npc.attack_timer =
            map.tick + Duration::try_milliseconds(npc_info.data.attack_wait).unwrap_or_default();
    }

    TargetingStage::move_to_movement(npc_info)
}

pub fn set_stage(store: &mut MapActorStore, key: GlobalKey, stage: NpcStages) {
    if let Some(npc) = store.npcs.get_mut(&key) {
        npc.stage = stage;
    }
}

pub fn npc_targeting<'a>(
    position: Position,
    npc_data: &NpcData,
    entity: Entity<'a>,
) -> Option<Entity<'a>> {
    let data = match &entity {
        Entity::Player(player) => Some((player.death, player.position)),
        Entity::Npc(npc) => {
            if npc_data.enemies.iter().any(|&x| npc.index == x) {
                Some((npc.death, npc.position))
            } else {
                None
            }
        }
        Entity::None => None,
    };

    if let Some((death, pos)) = data {
        let check = check_surrounding(position.map, pos.map, true);

        if death.is_alive()
            && position.checkdistance(pos.map_offset(check.into())) <= npc_data.sight
        {
            return Some(entity);
        }
    }

    None
}
