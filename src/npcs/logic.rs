use crate::{
    containers::{DeathType, Entity, Storage, World},
    gametypes::*,
    npcs::*,
    tasks::*,
};
use chrono::Duration;

pub fn update_npcs_targetting(
    world: &mut World,
    storage: &Storage,
    batch_index: usize,
) -> Result<()> {
    let start = 5 * batch_index;
    let end = start + 5;

    for index in start..end {
        let id = match storage.npc_ids.borrow().get_index(index) {
            Some(data) => *data,
            None => return Ok(()),
        };

        if let Some(Entity::Npc(n_data)) = world.get_opt_entity(id) {
            let (death_type, entity_index, map_pos) = {
                let n_data = n_data.try_lock()?;

                (
                    n_data.combat.death_type,
                    n_data.index,
                    n_data.movement.pos.map,
                )
            };

            if death_type == DeathType::Alive
                && let Some(npcdata) = storage.bases.npcs.get(entity_index as usize)
                && npcdata.can_target
                && match storage.maps.get(&map_pos) {
                    Some(map) => map.borrow().players_on_map(),
                    None => continue,
                }
            {
                targeting(world, storage, id, npcdata)?;
            }
        }
    }

    Ok(())
}

pub fn update_npcs_movement(
    world: &mut World,
    storage: &Storage,
    batch_index: usize,
) -> Result<()> {
    let tick = *storage.gettick.borrow();

    let start = 5 * batch_index;
    let end = start + 5;

    for index in start..end {
        let id = match storage.npc_ids.borrow().get_index(index) {
            Some(data) => *data,
            None => return Ok(()),
        };

        if let Some(Entity::Npc(n_data)) = world.get_opt_entity(id) {
            let (death_type, entity_index, movement_timer) = {
                let n_data = n_data.try_lock()?;
                (
                    n_data.combat.death_type,
                    n_data.index,
                    n_data.movement.move_timer,
                )
            };

            if death_type.is_alive()
                && let Some(npcdata) = storage.bases.npcs.get(entity_index as usize)
            {
                //movement
                if npcdata.can_move && movement_timer.0 <= tick {
                    npc_update_path(world, storage, id, npcdata)?;
                    npc_movement(world, storage, id, npcdata)?;
                    n_data.try_lock()?.movement.move_timer.0 = tick
                        + Duration::try_milliseconds(npcdata.movement_wait).unwrap_or_default();
                }
            }
        }
    }

    Ok(())
}

pub fn update_npcs_combat(world: &mut World, storage: &Storage, batch_index: usize) -> Result<()> {
    let tick = *storage.gettick.borrow();

    let start = 5 * batch_index;
    let end = start + 5;

    for index in start..end {
        let id = match storage.npc_ids.borrow().get_index(index) {
            Some(data) => *data,
            None => return Ok(()),
        };

        if let Some(Entity::Npc(n_data)) = world.get_opt_entity(id) {
            let (death_type, entity_index, map_pos, attack_timer) = {
                let n_data = n_data.try_lock()?;

                (
                    n_data.combat.death_type,
                    n_data.index,
                    n_data.movement.pos.map,
                    n_data.combat.attack_timer,
                )
            };

            if death_type.is_alive()
                && let Some(npcdata) = storage.bases.npcs.get(entity_index as usize)
            {
                //attacking
                if npcdata.can_attack
                    && match storage.maps.get(&map_pos) {
                        Some(map) => map.borrow().players_on_map(),
                        None => continue,
                    }
                    && attack_timer.0 <= tick
                {
                    npc_combat(world, storage, id, npcdata)?;

                    n_data.try_lock()?.combat.attack_timer.0 =
                        tick + Duration::try_milliseconds(npcdata.attack_wait).unwrap_or_default();
                }
            }
        }
    }

    Ok(())
}

pub fn update_npcs_spawn(world: &mut World, storage: &Storage, batch_index: usize) -> Result<()> {
    let tick = *storage.gettick.borrow();

    let start = 5 * batch_index;
    let end = start + 5;

    for index in start..end {
        let id = match storage.npc_ids.borrow().get_index(index) {
            Some(data) => *data,
            None => return Ok(()),
        };

        if let Some(Entity::Npc(n_data)) = world.get_opt_entity(id) {
            let (death_type, npc_despawn, despawn_timer, entity_index, spawn_timer, spawn_pos) = {
                let n_data = n_data.try_lock()?;

                (
                    n_data.combat.death_type,
                    n_data.despawns,
                    n_data.timer.despawntimer,
                    n_data.index,
                    n_data.timer.spawntimer,
                    n_data.movement.spawn.pos,
                )
            };

            match death_type {
                DeathType::Alive => {
                    if npc_despawn && despawn_timer <= tick {
                        n_data.try_lock()?.combat.death_type = DeathType::Dead;
                        storage.unload_npc.borrow_mut().push(id);
                        continue;
                    }

                    if let Some(npcdata) = storage.bases.npcs.get(entity_index as usize)
                        && !storage
                            .time
                            .borrow()
                            .in_range(npcdata.spawntime.0, npcdata.spawntime.1)
                    {
                        n_data.try_lock()?.combat.death_type = DeathType::Dead;
                        storage.unload_npc.borrow_mut().push(id);
                        continue;
                    }
                }
                DeathType::Dead => storage.unload_npc.borrow_mut().push(id),
                DeathType::Spawning => {
                    if spawn_timer < tick {
                        let map_data = match storage.maps.get(&spawn_pos.map) {
                            Some(map) => map,
                            None => continue,
                        };

                        let world_entity_type = world.get_kind(id)?;

                        //make sure we can spawn here before even spawning them.
                        if !map_data
                            .borrow()
                            .is_blocked_tile(spawn_pos, world_entity_type)
                        {
                            {
                                n_data.try_lock()?.combat.death_type = DeathType::Alive;
                            }
                            map_data.borrow_mut().add_entity_to_grid(spawn_pos);

                            DataTaskToken::NpcSpawn(spawn_pos.map)
                                .add_task(storage, npc_spawn_packet(world, id, true)?)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

pub fn unload_npcs(world: &mut World, storage: &Storage) -> Result<()> {
    if storage.unload_npc.borrow().is_empty() {
        return Ok(());
    }

    for i in storage.unload_npc.borrow().iter() {
        if let Some(Entity::Npc(n_data)) = world.get_opt_entity(*i) {
            let (zone_data, spawn_pos) = {
                let n_data = n_data.try_lock()?;

                (n_data.spawned_zone.0, n_data.movement.spawn)
            };

            let pos = storage.remove_npc(world, *i)?;

            if let Some(mapdata) = storage.maps.get(&spawn_pos.pos.map) {
                let mut data = mapdata.borrow_mut();

                data.remove_npc(*i);
                if let Some(zone) = zone_data {
                    data.zones[zone] = data.zones[zone].saturating_sub(1);
                }
            }
            DataTaskToken::EntityUnload(pos.map).add_task(storage, unload_entity_packet(*i)?)?;
        }
    }

    storage.unload_npc.borrow_mut().clear();

    Ok(())
}
