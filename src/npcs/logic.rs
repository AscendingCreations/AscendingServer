use crate::{
    containers::{DeathType, Entity, EntityKind, Storage, World},
    gametypes::*,
    npcs::*,
    tasks::*,
};
use chrono::Duration;

pub fn update_npcs(world: &mut World, storage: &Storage) -> Result<()> {
    let tick = *storage.gettick.borrow();
    let mut unloadnpcs = Vec::new();

    for id in &*storage.npc_ids.borrow() {
        if let Some(Entity::Npc(n_data)) = world.get_opt_entity(*id) {
            let (
                death_type,
                npc_index,
                spawn,
                pos,
                npctimer,
                npcdespawns,
                move_timer,
                attack_timer,
            ) = {
                let n_data = n_data.try_lock()?;

                (
                    n_data.combat.death_type,
                    n_data.index,
                    n_data.movement.spawn,
                    n_data.movement.pos,
                    n_data.timer,
                    n_data.despawns,
                    n_data.movement.move_timer,
                    n_data.combat.attack_timer,
                )
            };

            match death_type {
                DeathType::Alive => {
                    if npcdespawns && npctimer.despawntimer <= tick {
                        n_data.try_lock()?.combat.death_type = DeathType::Dead;
                        unloadnpcs.push(*id);
                        continue;
                    }

                    if let Some(npcdata) = storage.bases.npcs.get(npc_index as usize) {
                        if !storage
                            .time
                            .borrow()
                            .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                        {
                            n_data.try_lock()?.combat.death_type = DeathType::Dead;
                            unloadnpcs.push(*id);
                            continue;
                        }

                        //targeting
                        if npcdata.can_target
                            && match storage.maps.get(&pos.map) {
                                Some(map) => map.borrow().players_on_map(),
                                None => continue,
                            }
                        {
                            targeting(world, storage, *id, npcdata)?;
                        }

                        //movement
                        if npcdata.can_move && move_timer.0 <= tick {
                            npc_update_path(world, storage, *id, npcdata)?;
                            npc_movement(world, storage, *id, npcdata)?;
                            n_data.try_lock()?.movement.move_timer.0 = tick
                                + Duration::try_milliseconds(npcdata.movement_wait)
                                    .unwrap_or_default();
                        }

                        //attacking
                        if npcdata.can_attack
                            && match storage.maps.get(&pos.map) {
                                Some(map) => map.borrow().players_on_map(),
                                None => continue,
                            }
                            && attack_timer.0 <= tick
                        {
                            npc_combat(world, storage, *id, npcdata)?;

                            n_data.try_lock()?.combat.attack_timer.0 = tick
                                + Duration::try_milliseconds(npcdata.attack_wait)
                                    .unwrap_or_default();
                        }
                    }
                }
                DeathType::Dead => unloadnpcs.push(*id),
                DeathType::Spawning => {
                    if npctimer.spawntimer < tick {
                        let map_data = match storage.maps.get(&spawn.pos.map) {
                            Some(map) => map,
                            None => continue,
                        };

                        //make sure we can spawn here before even spawning them.
                        if !map_data
                            .borrow()
                            .is_blocked_tile(spawn.pos, EntityKind::Npc)
                        {
                            {
                                n_data.try_lock()?.combat.death_type = DeathType::Alive;
                            }
                            map_data.borrow_mut().add_entity_to_grid(spawn.pos);

                            DataTaskToken::NpcSpawn(spawn.pos.map)
                                .add_task(storage, npc_spawn_packet(world, *id, true)?)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    for i in unloadnpcs {
        if let Some(Entity::Npc(n_data)) = world.get_opt_entity(i) {
            let (zone_data, spawn_pos) = {
                let n_data = n_data.try_lock()?;

                (n_data.spawned_zone.0, n_data.movement.spawn)
            };

            let pos = storage.remove_npc(world, i)?;

            if let Some(mapdata) = storage.maps.get(&spawn_pos.pos.map) {
                let mut data = mapdata.borrow_mut();

                data.remove_npc(i);
                if let Some(zone) = zone_data {
                    data.zones[zone] = data.zones[zone].saturating_sub(1);
                }
            }
            DataTaskToken::EntityUnload(pos.map).add_task(storage, unload_entity_packet(i)?)?;
        }
    }

    Ok(())
}
