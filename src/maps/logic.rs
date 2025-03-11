use crate::{
    containers::{
        DeathType, DespawnTimer, Entity, EntityKind, GlobalKey, MapItem, MapItemEntity, Storage,
        World,
    },
    gametypes::*,
    items::Item,
    maps::is_dir_blocked,
    tasks::{DataTaskToken, map_item_packet},
};
use chrono::Duration;
use rand::{Rng, rng};
use std::{
    cmp::min,
    sync::{Arc, Mutex},
};

use super::check_surrounding;

pub fn update_maps(world: &mut World, storage: &Storage) -> Result<()> {
    let mut rng = rng();
    let mut spawnable = Vec::new();
    let mut len = storage.npc_ids.borrow().len();
    let tick = *storage.gettick.borrow();

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
                    if count >= NPCS_SPAWNCAP {
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
                            if rng.random_range(0..2) > 0 || !game_time.in_range(from, to) {
                                continue;
                            }

                            //Lets only allow spawning of a set amount each time. keep from over burdening the system.
                            if count >= max_spawnable || len >= MAX_WORLD_NPCS {
                                break;
                            }

                            let mut loop_count = 0;

                            //Only try to find a spot so many times randomly.
                            if !map.zonespawns[id].is_empty() {
                                while loop_count < 10 {
                                    let pos_id = rng.random_range(0..map.zonespawns[id].len());
                                    let (x, y) = map.zonespawns[id][pos_id];
                                    let spawn = Position::new(x as i32, y as i32, *position);

                                    loop_count += 1;

                                    //Check if the tile is blocked or not.
                                    if !data.is_blocked_tile(spawn, EntityKind::Npc) {
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
                }

                let mut data = map_data.borrow_mut();
                //Lets Spawn the npcs here;
                for (spawn, zone, npc_id) in spawnable.drain(..) {
                    if let Ok(Some(id)) = storage.add_npc(world, npc_id) {
                        data.add_npc(id);
                        data.zones[zone] = data.zones[zone].saturating_add(1);
                        spawn_npc(world, spawn, Some(zone), id)?;
                    }
                }
            }

            let mut add_items = Vec::new();

            for data in map_data.borrow_mut().spawnable_item.iter_mut() {
                let mut storage_mapitem = storage.map_items.borrow_mut();
                if !storage_mapitem.contains_key(&data.pos) {
                    if data.timer <= tick {
                        let map_item = create_mapitem(data.index, data.amount, data.pos);

                        let id = world.kinds.insert(EntityKind::MapItem);

                        world.entities.insert(
                            id,
                            Entity::MapItem(Arc::new(Mutex::new(MapItemEntity {
                                general: map_item,
                                despawn_timer: DespawnTimer::default(),
                            }))),
                        );

                        storage_mapitem.insert(data.pos, id);
                        DataTaskToken::ItemLoad(data.pos.map).add_task(
                            storage,
                            map_item_packet(
                                id,
                                map_item.pos,
                                map_item.item,
                                map_item.ownerid,
                                true,
                            )?,
                        )?;
                        add_items.push(id);
                    }
                } else {
                    data.timer = tick
                        + Duration::try_milliseconds(data.timer_set as i64).unwrap_or_default();
                }
            }

            for entity in add_items {
                map_data.borrow_mut().itemids.insert(entity);
            }
        }
    }

    Ok(())
}

pub fn create_mapitem(index: u32, value: u16, pos: Position) -> MapItem {
    MapItem {
        item: Item {
            num: index,
            val: value,
            ..Default::default()
        },
        despawn: None,
        ownertimer: None,
        ownerid: None,
        pos,
    }
}

pub fn spawn_npc(
    world: &mut World,
    pos: Position,
    zone: Option<usize>,
    entity: GlobalKey,
) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        n_data.movement.pos = pos;
        n_data.movement.spawn.pos = pos;
        n_data.spawned_zone.0 = zone;
        n_data.combat.death_type = DeathType::Spawning;
    }
    Ok(())
}

pub fn can_target(
    caster_pos: Position,
    target_pos: Position,
    target_death: DeathType,
    range: i32,
) -> bool {
    let check = check_surrounding(caster_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    range >= caster_pos.checkdistance(pos) && target_death.is_alive()
}

pub fn in_dir_attack_zone(
    storage: &Storage,
    caster_pos: Position,
    target_pos: Position,
    range: i32,
) -> bool {
    let check = check_surrounding(caster_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    if let Some(dir) = caster_pos.checkdirection(pos) {
        !is_dir_blocked(storage, caster_pos, dir as u8) && range >= caster_pos.checkdistance(pos)
    } else {
        false
    }
}
