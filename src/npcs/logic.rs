use crate::{containers::Storage, gametypes::*, npcs::*, tasks::*};
use chrono::Duration;
use unwrap_helpers::*;
use hecs::World;

pub fn update_npcs(world: &mut World, storage: &Storage) {
    let tick = *storage.gettick.borrow();
    let mut unloadnpcs = Vec::new();

    for id in &*storage.npc_ids.borrow() {
        let data = world.entity(id.0).expect("Could not get Entity");
        match *data.get::<&DeathType>().expect("Could not find DeathType") {
            DeathType::Alive => {
                if data.get::<&NpcDespawns>().expect("Could not find NpcDespawns").0 && 
                    data.get::<&NpcTimer>().expect("Could not find NpcTimer").despawntimer <= tick {
                    if let mut deathtype = data.get::<&mut DeathType>().expect("Could not find DeathType")
                        { *deathtype = DeathType::UnSpawned }
                    unloadnpcs.push(*id);
                    continue;
                }

                if let Some(npcdata) = storage.bases.npcs
                    .get(data.get::<&NpcIndex>().expect("Could not find NpcIndex").0 as usize) {
                    if !storage
                        .time
                        .borrow()
                        .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                    {
                        if let mut deathtype = data.get::<&mut DeathType>().expect("Could not find DeathType")
                            { *deathtype = DeathType::UnSpawned }
                        unloadnpcs.push(*id);
                        continue;
                    }

                    //targeting
                    if npcdata.can_target
                        && unwrap_continue!(storage.maps
                            .get(&data.get::<&Position>().expect("Could not find Position").map))
                            .borrow()
                            .players_on_map()
                    {
                        targeting(world, storage, id, npcdata);
                    }

                    //movement
                    if npcdata.can_move && data.get::<&MoveTimer>().expect("Could not find MoveTimer").0 <= tick {
                        npc_movement(world, storage, id, npcdata);
                        if let mut movetimer = data.get::<&mut MoveTimer>().expect("Could not find MoveTimer") 
                            { movetimer.0 = tick + Duration::milliseconds(npcdata.movement_wait) }
                    }

                    //attacking
                    if data.get::<&DeathType>().expect("Could not find DeathType").is_alive()
                        && npcdata.can_attack
                        && unwrap_continue!(storage.maps
                            .get(&data.get::<&Position>().expect("Could not find Position").map))
                            .borrow()
                            .players_on_map()
                        && data.get::<&AttackTimer>().expect("Could not find AttackTimer").0 < tick
                    {
                        npc_combat(world, storage, id, npcdata);
                    }

                    if data.get::<&InCombat>().expect("Could not find InCombat").0 &&
                        data.get::<&Combat>().expect("Could not find Combat").0 < tick {}
                }
            }

            DeathType::UnSpawned => unloadnpcs.push(*id),
            DeathType::Spawning => {
                if data.get::<&NpcTimer>().expect("Could not find MoveTimer").spawntimer < tick {
                    let map_data = unwrap_continue!(storage.maps
                        .get(&data.get::<&Spawn>().expect("Could not find Spawn").pos.map));

                    //make sure we can spawn here before even spawning them.
                    if map_data.borrow().is_blocked_tile(data.get::<&Spawn>().expect("Could not find Spawn").pos) {
                        if let mut deathtype = data.get::<&mut DeathType>().expect("Could not find DeathType")
                            { *deathtype = DeathType::Alive }
                        map_data.borrow_mut().add_entity_to_grid(data.get::<&Spawn>().expect("Could not find Spawn").pos);
                        let _ = DataTaskToken::NpcSpawn(data.get::<&Spawn>().expect("Could not find Spawn").pos.map)
                            .add_task(world, storage, &NpcSpawnPacket::new(world, id));
                    }
                }
            }
            _ => {}
        }
    }

    for i in unloadnpcs {
        if let Some(pos) = storage.remove_npc(world, i) {
            let _ =
                DataTaskToken::NpcUnload(pos.map).add_task(world, storage, &(i));
        }
    }
}
