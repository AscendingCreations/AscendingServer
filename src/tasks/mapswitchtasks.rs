use std::cmp::max;

use crate::{
    containers::{GameStore, GameWorld, HashSet},
    gametypes::*,
    maps::*,
    players::*,
    tasks::{map_item_packet, npc_spawn_packet, player_spawn_packet, DataTaskToken},
};

//types to buffer load when loading a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapSwitchTasks {
    Npc(Vec<Entity>),    //0
    Player(Vec<Entity>), //1
    Items(Vec<Entity>),  //2
}

pub async fn init_data_lists(
    world: &GameWorld,
    storage: &GameStore,
    user: &crate::Entity,
    oldmap: Option<MapPosition>,
) -> Result<()> {
    let mut map_switch_tasks = storage.map_switch_tasks.write().await;

    let (not_yet_sent_players, not_yet_sent_npcs, not_yet_sent_items) =
        if let Some(tasks) = map_switch_tasks.get_mut(user) {
            let mut player = HashSet::default();
            let mut npcs = HashSet::default();
            let mut items = HashSet::default();

            for task in tasks.drain(..) {
                match task {
                    MapSwitchTasks::Npc(v) => {
                        npcs = v.into_iter().collect();
                    }
                    MapSwitchTasks::Player(v) => {
                        player = v.into_iter().collect();
                    }
                    MapSwitchTasks::Items(v) => {
                        items = v.into_iter().collect();
                    }
                }
            }

            (player, npcs, items)
        } else {
            (HashSet::default(), HashSet::default(), HashSet::default())
        };

    //setup the old and new information so we know what to remove and add for.
    let mut old_players = HashSet::default();
    let mut old_npcs = HashSet::default();
    let mut old_items = HashSet::default();

    old_players.reserve(100);
    old_npcs.reserve(100);
    old_items.reserve(500);

    //create the data tasks to be ran against.
    let mut task_player = Vec::with_capacity(100);
    let mut task_npc = Vec::with_capacity(100);
    let mut task_item = Vec::with_capacity(600);

    //get the old map npcs, players and items so we can send remove requests.
    if let Some(old_map) = oldmap {
        for m in get_surrounding(old_map, true) {
            if let Some(map) = storage.maps.get(&m) {
                map.read().await.players.iter().for_each(|id| {
                    old_players.insert(*id);
                });

                map.read().await.npcs.iter().for_each(|id| {
                    old_npcs.insert(*id);
                });

                map.read().await.itemids.iter().for_each(|id| {
                    old_items.insert(*id);
                });
            }
        }
    }

    //Only get the New id's not in Old for the Vec we use the old data to deturmine what use to exist.
    //the users map is always first in the Vec of get_surrounding so it always gets loaded first.
    for m in get_surrounding(world.get_or_err::<Position>(user).await?.map, true) {
        if let Some(mapref) = storage.maps.get(&m) {
            let map = mapref.read().await;
            map.players.iter().for_each(|id| {
                if !old_players.contains(id) || not_yet_sent_players.contains(id) {
                    task_player.push(*id);
                }
            });

            map.npcs.iter().for_each(|id| {
                if !old_npcs.contains(id) || not_yet_sent_npcs.contains(id) {
                    task_npc.push(*id);
                }
            });

            map.itemids.iter().for_each(|id| {
                if !old_items.contains(id) || not_yet_sent_items.contains(id) {
                    task_item.push(*id);
                }
            });
        }
    }

    if let Some(tasks) = map_switch_tasks.get_mut(user) {
        tasks.push(MapSwitchTasks::Player(task_player));
        tasks.push(MapSwitchTasks::Npc(task_npc));
        tasks.push(MapSwitchTasks::Items(task_item));
    } else {
        map_switch_tasks.insert(
            *user,
            vec![
                MapSwitchTasks::Player(task_player),
                MapSwitchTasks::Npc(task_npc),
                MapSwitchTasks::Items(task_item),
            ],
        );
    }

    Ok(())
}

const PROCESS_LIMIT: usize = 1000;

pub async fn process_data_lists(world: &GameWorld, storage: &GameStore) -> Result<()> {
    let mut removals = Vec::new();
    let mut maptasks = storage.map_switch_tasks.write().await;
    let process_limit = max(PROCESS_LIMIT / (1 + maptasks.len() * 3), 10);

    for (entity, tasks) in maptasks.iter_mut() {
        let mut contains_data = false;

        let socket_id = {
            let lock = world.read().await;
            let id = lock.get::<&Socket>(entity.0).map(|s| s.id);
            id
        };

        if let Ok(socket_id) = socket_id {
            for task in tasks {
                let amount_left = match task {
                    MapSwitchTasks::Npc(entities) => {
                        let cursor = entities.len().saturating_sub(process_limit);

                        for entity in entities.drain(cursor..) {
                            if world.contains(&entity).await {
                                DataTaskToken::NpcSpawnToEntity(socket_id)
                                    .add_task(
                                        storage,
                                        npc_spawn_packet(world, &entity, false).await?,
                                    )
                                    .await?;
                            }
                        }

                        entities.len()
                    }
                    MapSwitchTasks::Player(entities) => {
                        let cursor = entities.len().saturating_sub(process_limit);

                        for entity in entities.drain(cursor..) {
                            if world.contains(&entity).await {
                                DataTaskToken::PlayerSpawnToEntity(socket_id)
                                    .add_task(
                                        storage,
                                        player_spawn_packet(world, &entity, false).await?,
                                    )
                                    .await?;
                            }
                        }

                        entities.len()
                    }
                    MapSwitchTasks::Items(entities) => {
                        let cursor = entities.len().saturating_sub(process_limit);

                        for entity in entities.drain(cursor..) {
                            let lock = world.read().await;
                            let map_item = lock.get::<&MapItem>(entity.0);
                            if let Ok(map_item) = map_item {
                                DataTaskToken::ItemLoadToEntity(socket_id)
                                    .add_task(
                                        storage,
                                        map_item_packet(
                                            entity,
                                            map_item.pos,
                                            map_item.item,
                                            map_item.ownerid,
                                            false,
                                        )?,
                                    )
                                    .await?;
                            }
                        }

                        entities.len()
                    }
                };

                if amount_left > 0 {
                    contains_data = true;
                }
            }
        }

        if !contains_data {
            removals.push(*entity);
        }
    }

    //we can now remove any empty tasks so we dont rerun them again.
    for entity in removals {
        maptasks.swap_remove(&entity);
    }

    Ok(())
}
