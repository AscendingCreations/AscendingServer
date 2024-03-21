use crate::{containers::Storage, gametypes::*, npcs::*, tasks::*};
use chrono::Duration;
use hecs::World;

pub fn update_npcs(world: &mut World, storage: &Storage) {
    let tick = *storage.gettick.borrow();
    let mut unloadnpcs = Vec::new();

    for id in &*storage.npc_ids.borrow() {
        match world.get_or_panic::<DeathType>(id) {
            DeathType::Alive => {
                if world.get_or_panic::<NpcDespawns>(id).0
                    && world.get_or_panic::<NpcTimer>(id).despawntimer <= tick
                {
                    *world
                        .get::<&mut DeathType>(id.0)
                        .expect("Could not find DeathType") = DeathType::Dead;
                    unloadnpcs.push(*id);
                    continue;
                }

                if let Some(npcdata) = storage
                    .bases
                    .npcs
                    .get(world.get_or_panic::<NpcIndex>(id).0 as usize)
                {
                    if !storage
                        .time
                        .borrow()
                        .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                    {
                        *world
                            .get::<&mut DeathType>(id.0)
                            .expect("Could not find DeathType") = DeathType::Dead;
                        unloadnpcs.push(*id);
                        continue;
                    }

                    //targeting
                    if npcdata.can_target
                        && match storage.maps.get(&world.get_or_panic::<Position>(id).map) {
                            Some(map) => map.borrow().players_on_map(),
                            None => continue,
                        }
                    {
                        targeting(world, storage, id, npcdata);
                    }

                    //movement
                    if npcdata.can_move && world.get_or_panic::<MoveTimer>(id).0 <= tick {
                        npc_movement(world, storage, id, npcdata);
                        world
                            .get::<&mut MoveTimer>(id.0)
                            .expect("Could not find MoveTimer")
                            .0 = tick
                            + Duration::try_milliseconds(npcdata.movement_wait).unwrap_or_default();
                    }

                    //attacking
                    if npcdata.can_attack
                        && match storage.maps.get(&world.get_or_panic::<Position>(id).map) {
                            Some(map) => map.borrow().players_on_map(),
                            None => continue,
                        }
                        && world.get_or_panic::<AttackTimer>(id).0 <= tick
                    {
                        npc_combat(world, storage, id, npcdata);

                        world
                            .get::<&mut AttackTimer>(id.0)
                            .expect("Could not find AttackTimer")
                            .0 = tick
                            + Duration::try_milliseconds(npcdata.attack_wait).unwrap_or_default();
                    }

                    if world.get_or_panic::<InCombat>(id).0
                        && world.get_or_panic::<Combat>(id).0 < tick
                    {}
                }
            }
            DeathType::Dead => unloadnpcs.push(*id),
            DeathType::Spawning => {
                if world.get_or_panic::<NpcTimer>(id).spawntimer < tick {
                    let map_data = match storage.maps.get(&world.get_or_panic::<Spawn>(id).pos.map)
                    {
                        Some(map) => map,
                        None => continue,
                    };

                    //make sure we can spawn here before even spawning them.
                    if !map_data
                        .borrow()
                        .is_blocked_tile(world.get_or_panic::<Spawn>(id).pos)
                    {
                        {
                            *world
                                .get::<&mut DeathType>(id.0)
                                .expect("Could not find DeathType") = DeathType::Alive;
                        }
                        map_data
                            .borrow_mut()
                            .add_entity_to_grid(world.get_or_panic::<Spawn>(id).pos);

                        let _ = DataTaskToken::NpcSpawn(world.get_or_panic::<Spawn>(id).pos.map)
                            .add_task(storage, &NpcSpawnPacket::new(world, id, true));
                    }
                }
            }
            _ => {}
        }
    }

    for i in unloadnpcs {
        let zone_data = world.get_or_panic::<NpcSpawnedZone>(&i).0;

        if let Some(pos) = storage.remove_npc(world, i) {
            if let Some(mapdata) = storage.maps.get(&pos.map) {
                let mut data = mapdata.borrow_mut();

                data.remove_npc(i);
                if let Some(zone) = zone_data {
                    data.zones[zone] = data.zones[zone].saturating_sub(1);
                }
            }
            let _ = DataTaskToken::EntityUnload(pos.map).add_task(storage, &(i));
        }
    }
}
