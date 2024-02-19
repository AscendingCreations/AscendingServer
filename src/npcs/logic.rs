use crate::{containers::Storage, gametypes::*, npcs::*, tasks::*};
use chrono::Duration;
use hecs::World;
use unwrap_helpers::*;

pub fn update_npcs(world: &mut World, storage: &Storage) {
    let tick = *storage.gettick.borrow();
    let mut unloadnpcs = Vec::new();

    for id in &*storage.npc_ids.borrow() {
        match world.get_or_panic::<&DeathType>(&id) {
            DeathType::Alive => {
                if world.get_or_panic::<&NpcDespawns>(&id).0
                    && world.get_or_panic::<&NpcTimer>(&id).despawntimer <= tick
                {
                    let mut deathtype = world
                        .get::<&mut DeathType>(id.0)
                        .expect("Could not find DeathType");
                    *deathtype = DeathType::UnSpawned;
                    unloadnpcs.push(*id);
                    continue;
                }

                if let Some(npcdata) = storage
                    .bases
                    .npcs
                    .get(world.get_or_panic::<&NpcIndex>(&id).0 as usize)
                {
                    if !storage
                        .time
                        .borrow()
                        .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                    {
                        let mut deathtype = world
                            .get::<&mut DeathType>(id.0)
                            .expect("Could not find DeathType");
                        *deathtype = DeathType::UnSpawned;
                        unloadnpcs.push(*id);
                        continue;
                    }

                    //targeting
                    if npcdata.can_target
                        && unwrap_continue!(storage
                            .maps
                            .get(&world.get_or_panic::<&Position>(&id).map))
                        .borrow()
                        .players_on_map()
                    {
                        targeting(world, storage, id, npcdata);
                    }

                    //movement
                    if npcdata.can_move && world.get_or_panic::<&MoveTimer>(&id).0 <= tick {
                        npc_movement(world, storage, id, npcdata);
                        world
                            .get::<&mut MoveTimer>(id.0)
                            .expect("Could not find MoveTimer")
                            .0 = tick + Duration::milliseconds(npcdata.movement_wait);
                    }

                    //attacking
                    if npcdata.can_attack
                        && unwrap_continue!(storage
                            .maps
                            .get(&world.get_or_panic::<&Position>(&id).map))
                        .borrow()
                        .players_on_map()
                        && world.get_or_panic::<&AttackTimer>(&id).0 < tick
                    {
                        npc_combat(world, storage, id, npcdata);
                    }

                    if world.get_or_panic::<&InCombat>(&id).0
                        && world.get_or_panic::<&Combat>(&id).0 < tick
                    {}
                }
            }

            DeathType::UnSpawned => unloadnpcs.push(*id),
            DeathType::Spawning => {
                if world.get_or_panic::<&NpcTimer>(&id).spawntimer < tick {
                    let map_data = unwrap_continue!(storage
                        .maps
                        .get(&world.get_or_panic::<&Spawn>(&id).pos.map));

                    //make sure we can spawn here before even spawning them.
                    if map_data
                        .borrow()
                        .is_blocked_tile(world.get_or_panic::<&Spawn>(&id).pos)
                    {
                        //Sherwin: We can Set parts between {} to let the system know any data REF loaded here is unload before }
                        {
                            *world
                                .get::<&mut DeathType>(id.0)
                                .expect("Could not find DeathType") = DeathType::Alive;
                        }
                        map_data
                            .borrow_mut()
                            .add_entity_to_grid(world.get_or_panic::<&Spawn>(&id).pos);
                        let _ = DataTaskToken::NpcSpawn(world.get_or_panic::<&Spawn>(&id).pos.map)
                            .add_task(storage, &NpcSpawnPacket::new(world, id));
                    }
                }
            }
            _ => {}
        }
    }

    //Sherwin: This is to unload all Dead npocs or NPC that need to just be despawned.
    // We do this After everything else so we can easily get an entire count of it.
    for i in unloadnpcs {
        if let Some(pos) = storage.remove_npc(world, i) {
            let _ = DataTaskToken::NpcUnload(pos.map).add_task(storage, &(i));
        }
    }
}
