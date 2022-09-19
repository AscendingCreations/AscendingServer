use crate::{containers::Storage, gameloop::*, gametypes::*, npcs::*};
use chrono::Duration;
use unwrap_helpers::*;

pub fn update_npcs(world: &Storage) {
    let tick = *world.gettick.borrow();
    let mut unloadnpcs = Vec::new();

    for i in &*world.npc_ids.borrow() {
        if let Some(npc) = world.npcs.borrow().get(*i) {
            let mut npc = npc.borrow_mut();

            match npc.e.life {
                DeathType::Alive => {
                    if npc.despawns && npc.despawntimer <= tick {
                        npc.e.life = DeathType::UnSpawned;
                        unloadnpcs.push(*i);
                        continue;
                    }

                    if let Some(npcdata) = world.bases.npcs.get(npc.num as usize) {
                        if !world
                            .time
                            .borrow()
                            .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                        {
                            npc.e.life = DeathType::UnSpawned;
                            continue;
                        }

                        //targeting
                        if npcdata.can_target
                            && unwrap_continue!(world.maps.get(&npc.e.pos.map))
                                .borrow()
                                .players_on_map()
                        {
                            targeting(world, &mut npc, npcdata);
                        }

                        //movement
                        if npcdata.can_move && npc.e.movetimer <= tick {
                            movement(world, &mut npc, npcdata);
                            npc.e.movetimer = tick + Duration::milliseconds(npcdata.movement_wait);
                        }

                        //attacking
                        if npc.e.life.is_alive()
                            && npcdata.can_attack
                            && unwrap_continue!(world.maps.get(&npc.e.pos.map))
                                .borrow()
                                .players_on_map()
                            && npc.e.attacktimer < tick
                        {
                            npc_combat(world, &mut npc, npcdata);
                        }

                        if npc.e.incombat && npc.e.combattimer < tick {}
                    }
                }

                DeathType::UnSpawned => unloadnpcs.push(*i),
                DeathType::Spawning => {
                    if npc.spawntimer < tick {
                        let map_data = unwrap_continue!(world.maps.get(&npc.e.spawn.map));

                        //make sure we can spawn here before even spawning them.
                        if map_data.borrow().is_blocked_tile(npc.e.spawn) {
                            npc.e.life = DeathType::Alive;
                            map_data.borrow_mut().add_entity_to_grid(npc.e.spawn);
                            //TODO: Send npc to maps.packets or buffer of packets per map.
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for i in unloadnpcs {
        if let Some(npc) = world.remove_npc(i) {
            let _ = send_data_remove(world, i as u64, npc.e.pos.map, 0);
        }
    }
}