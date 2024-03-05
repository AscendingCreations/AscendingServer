use crate::{containers::Storage, gametypes::*};
use rand::{thread_rng, Rng};
use std::cmp::min;

pub fn update_maps(world: &mut hecs::World, storage: &Storage) -> Result<()> {
    let mut rng = thread_rng();
    let mut spawnable = Vec::new();
    let mut len = storage.npc_ids.borrow().len();

    for (position, map_data) in &storage.maps {
        // Only Spawn is a player is on or near a the map.
        if map_data.borrow().players_on_map() {
            //get this so we can Add to it each time without needing to borrow() npcs again.

            let mut count = 0;

            //Spawn NPC's if the max npc's per world is not yet reached.
            if len < MAX_WORLD_NPCS {
                let map = storage
                    .bases
                    .maps
                    .get(position)
                    .ok_or(AscendingError::MapNotFound(*position))?;

                for (id, (max_npcs, zone_npcs)) in map.zones.iter().enumerate() {
                    let data = map_data.borrow();
                    //We want to only allow this many npcs per map to spawn at a time.
                    if count + 1 >= NPCS_SPAWNCAP {
                        break;
                    }

                    if !map.zonespawns[id].is_empty() && data.zones[id] < *max_npcs {
                        // Set the Max allowed to spawn by either spawn cap or npc spawn limit.
                        let max_spawnable =
                            min((*max_npcs - data.zones[id]) as usize, NPCS_SPAWNCAP);

                        //Lets Do this for each npc;
                        for npc_id in zone_npcs
                            .iter()
                            .filter(|v| v.is_some())
                            .map(|v| v.unwrap_or_default())
                        {
                            let game_time = storage.time.borrow();
                            let (from, to) = storage
                                .bases
                                .npcs
                                .get(npc_id as usize)
                                .ok_or(AscendingError::NpcNotFound(npc_id))?
                                .spawntime;

                            //Give them a percentage chance to actually spawn
                            //or see if we can spawn them yet within the time frame.
                            if rng.gen_range(0..2) > 0 || !game_time.in_range(from, to) {
                                continue;
                            }

                            //Lets only allow spawning of a set amount each time. keep from over burdening the system.
                            if count + 1 >= max_spawnable || len + 1 >= MAX_WORLD_NPCS {
                                break;
                            }

                            let mut loop_count = 0;

                            //Only try to find a spot so many times randomly.
                            while loop_count < 10 {
                                let pos_id = rng.gen_range(0..map.zonespawns[id].len());
                                let (x, y) = map.zonespawns[id][pos_id];
                                let spawn = Position::new(x as i32, y as i32, *position);

                                loop_count += 1;

                                //Check if the tile is blocked or not.
                                if !data.is_blocked_tile(spawn) {
                                    //Set NPC as spawnable and to do further checks later.
                                    //Doing this to make the code more readable.
                                    spawnable.push((spawn, id, npc_id));
                                    count = count.saturating_add(1);
                                    len = len.saturating_add(1);
                                    break;
                                }
                            }
                        }
                    }
                }

                let mut data = map_data.borrow_mut();
                //Lets Spawn the npcs here;
                for (_spawn, _zone, npc_id) in spawnable.drain(..) {
                    if let Ok(id) = storage.add_npc(world, npc_id) {
                        data.add_npc(id);
                    }
                }
            }
        }
    }

    Ok(())
}
