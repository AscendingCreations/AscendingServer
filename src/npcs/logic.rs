use crate::{
    gametypes::*,
    maps::{MapActor, MapActorStore},
    npcs::*,
    tasks::*,
};

pub async fn update_npc_states(map: &mut MapActor, store: &mut MapActorStore) -> Result<()> {
    let mut unload_npcs = Vec::new();

    for (key, npc) in store.npcs.iter_mut() {
        match npc.death {
            Death::Alive => {
                if npc.despawn && npc.despawn_timer <= map.tick {
                    unload_npcs.push(*key);
                    continue;
                }

                if let Some(npc_data) = map.storage.get_npc(npc.index) {
                    if !store
                        .time
                        .in_range(npc_data.spawntime.0, npc_data.spawntime.1)
                    {
                        unload_npcs.push(*key);
                        continue;
                    }

                    //start here first
                    if npc_data.can_target {
                        let info = NpcInfo::new(npc.key, npc.position, npc_data.clone());
                        npc.stage = NpcStages::Targeting;

                        if let Some(position) = npc.target.get_pos() {
                            if position.map == npc.position.map {
                                map.npc_state_machine
                                    .push_back(TargetingStage::check_target(info, npc.target));
                            } else {
                                //target on another map so we send it there first!
                                let sender = map
                                    .storage
                                    .map_senders
                                    .get_mut(&position.map)
                                    .expect("Missing map?");

                                sender
                                    .send(crate::maps::MapIncomming::NpcStage {
                                        map_id: npc.position.map,
                                        stage: TargetingStage::check_target(info, npc.target),
                                    })
                                    .await
                                    .unwrap();
                            }
                        } else {
                            // we have no target so lets try and get one
                            map.npc_state_machine
                                .push_back(TargetingStage::get_target_maps(info));
                        }

                        continue;
                    }

                    //movement
                    if npc_data.can_move {
                        let info = NpcInfo::new(npc.key, npc.position, npc_data.clone());
                        map.npc_state_machine
                            .push_back(MovementStage::path_start(info));
                        npc.stage = NpcStages::Movement;
                    }

                    //attacking
                    if npc_data.can_attack {
                        let info = NpcInfo::new(npc.key, npc.position, npc_data.clone());
                        map.npc_state_machine
                            .push_back(CombatStage::behaviour_check(info));
                        npc.stage = NpcStages::Combat;
                    }
                }
            }
            Death::Dead => unload_npcs.push(*key),
            Death::Spawning => {
                if npc.spawn_timer < map.tick {
                    //make sure we can spawn here before even spawning them.
                    if !map.is_blocked_tile(npc.spawn_pos, WorldEntityType::Npc) {
                        npc.death = Death::Alive;

                        map.add_entity_to_grid(npc.spawn_pos);

                        DataTaskToken::NpcSpawn.add_task(map, npc_spawn_packet(npc, true)?)?;
                    }
                }
            }
            _ => {}
        }
    }

    for key in unload_npcs {
        if let Some(npc) = store.remove_npc(key) {
            map.remove_entity_from_grid(npc.position);

            if let Some(zone) = npc.spawn_zone {
                map.zones[zone] = map.zones[zone].saturating_sub(1);
            }

            DataTaskToken::EntityUnload.add_task(map, unload_entity_packet(key)?)?;
        }
    }

    Ok(())
}

pub async fn npc_state_handler(map: &mut MapActor, store: &mut MapActorStore) -> Result<()> {
    while let Some(stage) = map.npc_state_machine.pop_front() {
        let next_stage = match stage {
            NpcStage::None(npc_info) => {
                if let Some(npc) = store.npcs.get_mut(&npc_info.key) {
                    npc.stage = NpcStages::None;
                }

                continue;
            }
            NpcStage::Continue => continue,
            NpcStage::Targeting(targeting_stage) => {
                npc_targeting(map, store, targeting_stage).await?
            }
            NpcStage::Combat(combat_stage) => todo!(),
            NpcStage::Movement(movement_stage) => todo!(),
        };

        next_stage.send(map).await;
    }

    Ok(())
}

pub async fn npc_targeting(
    map: &mut MapActor,
    store: &mut MapActorStore,
    stage: TargetingStage,
) -> Result<NpcStage> {
    let stage = match stage {
        TargetingStage::CheckTarget { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                targeting::check_target(store, npc_info, target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::NpcDeTargetChance { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                targeting::check_detargeting(map, npc_info, target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::CheckDistance { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                targeting::check_target_distance(store, npc_info, target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::ClearTarget { npc_info } => {
            if !npc_info.is_dead(map, store) {
                targeting::clear_target(store, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::GetTargetMaps { npc_info } => {
            if !npc_info.is_dead(map, store) {
                targeting::get_targeting_maps(map, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::GetTargetFromMaps { npc_info, mut maps } => {
            if !npc_info.is_dead(map, store) {
                let stage = targeting::get_target(store, npc_info);

                if let NpcStage::Targeting(TargetingStage::MoveToMovement { npc_info }) = stage {
                    if let Some(next_map) = maps.pop() {
                        send_stage(
                            map,
                            next_map,
                            TargetingStage::get_target_from_maps(npc_info, maps),
                        )
                        .await;
                        NpcStage::Continue
                    } else {
                        TargetingStage::move_to_movement(npc_info)
                    }
                } else {
                    stage
                }
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::SetTarget {
            npc_info,
            target,
            target_pos,
        } => {
            if !npc_info.is_dead(map, store) {
                targeting::set_target(map, store, npc_info, target, target_pos)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::MoveToMovement { npc_info } => {
            if !npc_info.is_dead(map, store) {
                if let Some(npc) = store.npcs.get_mut(&npc_info.key) {
                    if npc_info.data.can_move {
                        npc.stage = NpcStages::Movement;
                        MovementStage::path_start(npc_info)
                    } else if npc_info.data.can_attack {
                        npc.stage = NpcStages::Combat;
                        CombatStage::behaviour_check(npc_info)
                    } else {
                        NpcStage::None(npc_info)
                    }
                } else {
                    NpcStage::None(npc_info)
                }
            } else {
                NpcStage::None(npc_info)
            }
        }
    };

    Ok(stage)
}

pub async fn send_stage(map: &MapActor, map_pos: MapPosition, stage: NpcStage) {
    let sender = map.storage.map_senders.get(&map_pos).expect("Missing map?");

    sender
        .send(crate::maps::MapIncomming::NpcStage {
            map_id: map.position,
            stage,
        })
        .await
        .expect("Could not send to map. means map got unloaded?");
}
