use crate::{
    gametypes::*,
    maps::{MapActor, MapActorStore},
    npcs::*,
    tasks::*,
    GlobalKey,
};
use chrono::Duration;

pub async fn update_npcs(map: &mut MapActor, store: &mut MapActorStore) -> Result<()> {
    let mut unloadnpcs = Vec::new();
    let npc_ids = store
        .npcs
        .iter()
        .map(|(key, npc)| key)
        .copied()
        .collect::<Vec<GlobalKey>>();

    for id in npc_ids {
        let npc = store.npcs.get_mut(&id);

        if let Some(npc) = npc {
            match npc.death {
                Death::Alive => {
                    if npc.despawn && npc.despawn_timer <= map.tick {
                        unloadnpcs.push(id);
                        continue;
                    }

                    if let Some(npcdata) = map.storage.get_npc(npc.index) {
                        if !store
                            .time
                            .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                        {
                            unloadnpcs.push(id);
                            continue;
                        }

                        //targeting
                        if npcdata.can_target {
                            //targeting(world, storage, id, npcdata).await?;
                        }

                        //movement
                        if npcdata.can_move && npc.move_timer <= map.tick {
                            //npc_update_path(world, storage, id, npcdata).await?;
                            //npc_movement(world, storage, id, npcdata).await?;
                            npc.move_timer = map.tick
                                + Duration::try_milliseconds(npcdata.movement_wait)
                                    .unwrap_or_default();
                        }

                        //attacking
                        if npcdata.can_attack && npc.attack_timer <= map.tick {
                            //npc_combat(world, storage, id, npcdata).await?;

                            npc.attack_timer = map.tick
                                + Duration::try_milliseconds(npcdata.attack_wait)
                                    .unwrap_or_default();
                        }
                    }
                }
                Death::Dead => unloadnpcs.push(id),
                Death::Spawning => {
                    if npc.spawn_timer < map.tick {
                        //make sure we can spawn here before even spawning them.
                        if !map.is_blocked_tile(npc.spawn_pos, WorldEntityType::Npc) {
                            npc.death = Death::Alive;

                            map.add_entity_to_grid(npc.spawn_pos);

                            DataTaskToken::NpcSpawn.add_task(map, npc_spawn_packet(&npc, true)?)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for i in unloadnpcs {
        if let Some(npc) = store.remove_npc(i) {
            map.remove_entity_from_grid(npc.position);

            if let Some(zone) = npc.spawn_zone {
                map.zones[zone] = map.zones[zone].saturating_sub(1);
            }

            DataTaskToken::EntityUnload.add_task(map, unload_entity_packet(i)?)?;
        }
    }

    Ok(())
}
