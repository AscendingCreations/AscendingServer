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
        let npc = store.npcs.get(&id).cloned();

        match world.get_or_err::<Death>(id).await? {
            Death::Alive => {
                if world.get_or_err::<NpcDespawns>(id).await?.0
                    && world.get_or_err::<NpcTimer>(id).await?.despawntimer <= tick
                {
                    let lock = world.write().await;
                    *lock.get::<&mut Death>(id.0)? = Death::Dead;
                    unloadnpcs.push(*id);
                    continue;
                }

                if let Some(npcdata) = storage
                    .bases
                    .npcs
                    .get(world.get_or_err::<NpcIndex>(id).await?.0 as usize)
                {
                    if !storage
                        .time
                        .read()
                        .await
                        .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                    {
                        let lock = world.write().await;
                        *lock.get::<&mut Death>(id.0)? = Death::Dead;
                        unloadnpcs.push(*id);
                        continue;
                    }

                    //targeting
                    if npcdata.can_target
                        && match storage
                            .maps
                            .get(&world.get_or_err::<Position>(id).await?.map)
                        {
                            Some(map) => map.read().await.players_on_map(),
                            None => continue,
                        }
                    {
                        targeting(world, storage, id, npcdata).await?;
                    }

                    //movement
                    if npcdata.can_move && world.get_or_err::<MoveTimer>(id).await?.0 <= tick {
                        npc_update_path(world, storage, id, npcdata).await?;
                        npc_movement(world, storage, id, npcdata).await?;
                        let lock = world.write().await;
                        lock.get::<&mut MoveTimer>(id.0)?.0 = tick
                            + Duration::try_milliseconds(npcdata.movement_wait).unwrap_or_default();
                    }

                    //attacking
                    if npcdata.can_attack
                        && match storage
                            .maps
                            .get(&world.get_or_err::<Position>(id).await?.map)
                        {
                            Some(map) => map.read().await.players_on_map(),
                            None => continue,
                        }
                        && world.get_or_err::<AttackTimer>(id).await?.0 <= tick
                    {
                        npc_combat(world, storage, id, npcdata).await?;
                        let lock = world.write().await;
                        lock.get::<&mut AttackTimer>(id.0)?.0 = tick
                            + Duration::try_milliseconds(npcdata.attack_wait).unwrap_or_default();
                    }

                    if world.get_or_err::<InCombat>(id).await?.0
                        && world.get_or_err::<Combat>(id).await?.0 < tick
                    {}
                }
            }
            Death::Dead => unloadnpcs.push(*id),
            Death::Spawning => {
                if world.get_or_err::<NpcTimer>(id).await?.spawntimer < tick {
                    let map_data = match storage
                        .maps
                        .get(&world.get_or_err::<Spawn>(id).await?.pos.map)
                    {
                        Some(map) => map,
                        None => continue,
                    };

                    //make sure we can spawn here before even spawning them.
                    if !map_data.read().await.is_blocked_tile(
                        world.get_or_err::<Spawn>(id).await?.pos,
                        WorldEntityType::Npc,
                    ) {
                        {
                            let lock = world.write().await;
                            *lock.get::<&mut Death>(id.0)? = Death::Alive;
                        }
                        map_data
                            .write()
                            .await
                            .add_entity_to_grid(world.get_or_err::<Spawn>(id).await?.pos);

                        DataTaskToken::NpcSpawn(world.get_or_err::<Spawn>(id).await?.pos.map)
                            .add_task(storage, npc_spawn_packet(world, id, true).await?)
                            .await?;
                    }
                }
            }
            _ => {}
        }
    }

    for i in unloadnpcs {
        let zone_data = world.get_or_err::<NpcSpawnedZone>(&i).await?.0;
        let spawn_pos = world.get_or_err::<Spawn>(&i).await?;
        let pos = storage.remove_npc(world, i).await?;

        if let Some(mapdata) = storage.maps.get(&spawn_pos.pos.map) {
            let mut data = mapdata.write().await;

            data.remove_npc(i);
            if let Some(zone) = zone_data {
                data.zones[zone] = data.zones[zone].saturating_sub(1);
            }
        }

        DataTaskToken::EntityUnload(pos.map)
            .add_task(storage, unload_entity_packet(i)?)
            .await?;
    }

    Ok(())
}
