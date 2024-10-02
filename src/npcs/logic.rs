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
                //make sure we can spawn here before even spawning them.
                if npc.spawn_timer < map.tick
                    && !map.is_blocked_tile(npc.spawn_pos, WorldEntityType::Npc)
                {
                    npc.death = Death::Alive;

                    map.add_entity_to_grid(npc.spawn_pos);

                    DataTaskToken::NpcSpawn.add_task(map, npc_spawn_packet(npc, true)?)?;
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
                stages::npc_targeting(map, store, targeting_stage).await
            }
            NpcStage::Combat(combat_stage) => stages::npc_combat(map, store, combat_stage).await,
            NpcStage::Movement(movement_stage) => {
                stages::npc_movement(map, store, movement_stage).await
            }
        };

        match next_stage {
            Ok(next) => next.send(map).await,
            Err(e) => log::error!("Error in npc_state_handler: {}", e),
        };
    }

    Ok(())
}
