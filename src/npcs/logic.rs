use crate::{containers::Storage, gametypes::*, npcs::*, tasks::*};
use chrono::Duration;
use hecs::World;

pub async fn update_npcs(world: &mut World, storage: &Storage) -> Result<()> {
    let tick = *storage.gettick.borrow();
    let mut unloadnpcs = Vec::new();

    for id in &*storage.npc_ids.borrow() {
        match world.get_or_err::<DeathType>(id)? {
            DeathType::Alive => {
                if world.get_or_err::<NpcDespawns>(id)?.0
                    && world.get_or_err::<NpcTimer>(id)?.despawntimer <= tick
                {
                    *world.get::<&mut DeathType>(id.0)? = DeathType::Dead;
                    unloadnpcs.push(*id);
                    continue;
                }

                if let Some(npcdata) = storage
                    .bases
                    .npcs
                    .get(world.get_or_err::<NpcIndex>(id)?.0 as usize)
                {
                    if !storage
                        .time
                        .borrow()
                        .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                    {
                        *world.get::<&mut DeathType>(id.0)? = DeathType::Dead;
                        unloadnpcs.push(*id);
                        continue;
                    }

                    //targeting
                    if npcdata.can_target
                        && match storage.maps.get(&world.get_or_err::<Position>(id)?.map) {
                            Some(map) => map.borrow().players_on_map(),
                            None => continue,
                        }
                    {
                        targeting(world, storage, id, npcdata).await?;
                    }

                    //movement
                    if npcdata.can_move && world.get_or_err::<MoveTimer>(id)?.0 <= tick {
                        npc_update_path(world, storage, id, npcdata).await?;
                        npc_movement(world, storage, id, npcdata).await?;
                        world.get::<&mut MoveTimer>(id.0)?.0 = tick
                            + Duration::try_milliseconds(npcdata.movement_wait).unwrap_or_default();
                    }

                    //attacking
                    if npcdata.can_attack
                        && match storage.maps.get(&world.get_or_err::<Position>(id)?.map) {
                            Some(map) => map.borrow().players_on_map(),
                            None => continue,
                        }
                        && world.get_or_err::<AttackTimer>(id)?.0 <= tick
                    {
                        npc_combat(world, storage, id, npcdata).await?;

                        world.get::<&mut AttackTimer>(id.0)?.0 = tick
                            + Duration::try_milliseconds(npcdata.attack_wait).unwrap_or_default();
                    }

                    if world.get_or_err::<InCombat>(id)?.0
                        && world.get_or_err::<Combat>(id)?.0 < tick
                    {}
                }
            }
            DeathType::Dead => unloadnpcs.push(*id),
            DeathType::Spawning => {
                if world.get_or_err::<NpcTimer>(id)?.spawntimer < tick {
                    let map_data = match storage.maps.get(&world.get_or_err::<Spawn>(id)?.pos.map) {
                        Some(map) => map,
                        None => continue,
                    };

                    //make sure we can spawn here before even spawning them.
                    if !map_data
                        .borrow()
                        .is_blocked_tile(world.get_or_err::<Spawn>(id)?.pos, WorldEntityType::Npc)
                    {
                        {
                            *world.get::<&mut DeathType>(id.0)? = DeathType::Alive;
                        }
                        map_data
                            .borrow_mut()
                            .add_entity_to_grid(world.get_or_err::<Spawn>(id)?.pos);

                        DataTaskToken::NpcSpawn(world.get_or_err::<Spawn>(id)?.pos.map)
                            .add_task(storage, npc_spawn_packet(world, id, true)?)
                            .await?;
                    }
                }
            }
            _ => {}
        }
    }

    for i in unloadnpcs {
        let zone_data = world.get_or_err::<NpcSpawnedZone>(&i)?.0;
        let spawn_pos = world.get_or_err::<Spawn>(&i)?;
        let pos = storage.remove_npc(world, i)?;

        if let Some(mapdata) = storage.maps.get(&spawn_pos.pos.map) {
            let mut data = mapdata.borrow_mut();

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
