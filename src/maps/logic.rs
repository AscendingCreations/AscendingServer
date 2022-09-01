use crate::{containers::Storage, gameloop::*, gametypes::*, maps::*, players::*, sql::*};
use chrono::Duration;
use rand::{thread_rng, Rng};
use std::cmp;
pub fn update_maps(world: &Storage) -> Result<()> {
    let tick = *world.gettick.borrow();

    for (position, map_data) in &world.map_data {
        if map_data.borrow().players_on_map() {
            //Spawn NPC's if the max npc's per world is not yet reached.
            if world.npcs.borrow().len() < MAX_WORLD_NPCS {
                let map = world
                    .bases
                    .map
                    .get(position)
                    .ok_or(AscendingError::MapNotFound(*position))?;

                for (id, (max, zone_npcs)) in map.zones.iter().enumerate() {
                    if map_data.borrow().zones[id] < *max {
                        for npc in zone_npcs {
                            if let Some(npc_id) = *npc {
                                let mut rng = thread_rng();
                                let pos_id = rng.gen_range(0..map.zonespawns[id].len());
                                //get the x,y position to spawn the npc too!
                                let pos = map.zonespawns[id][pos_id];
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
