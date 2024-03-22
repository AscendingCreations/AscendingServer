use crate::{
    containers::Storage,
    gameloop::*,
    gametypes::*,
    maps::*,
    players::*,
    tasks::{DataTaskToken, MapItemPacket, NpcSpawnPacket, PlayerSpawnPacket},
};
use hecs::World;
use indexmap::IndexSet;

//types to buffer load when loading a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapSwitchTasks {
    Npc(Vec<Entity>),    //0
    Player(Vec<Entity>), //1
    Items(Vec<Entity>),  //2
}

pub fn init_data_lists(
    world: &mut World,
    storage: &Storage,
    user: &crate::Entity,
    oldmap: Option<MapPosition>,
) {
    let mut map_switch_tasks = storage.map_switch_tasks.borrow_mut();

    //Remove old tasks and replace with new ones during map switching.
    if let Some(tasks) = map_switch_tasks.get_mut(user) {
        //If this contains any tasks we will clear them first. as we only want to send whats relevent.
        tasks.clear();
    } else {
        //if the task was removed after processing then we simply add a new one.
        map_switch_tasks.insert(*user, vec![]);
    }

    let socket_id = world.get::<&Socket>(user.0).unwrap().id;

    // Lets remove any lengering Packet Sends if they still Exist.
    {
        let mut packet_cache = storage.packet_cache.borrow_mut();
        let mut packet_cache_ids = storage.packet_cache_ids.borrow_mut();
        for key in [
            DataTaskToken::NpcSpawnToEntity(socket_id),
            DataTaskToken::PlayerSpawnToEntity(socket_id),
            DataTaskToken::ItemLoadToEntity(socket_id),
        ] {
            packet_cache.swap_remove(&key);
            packet_cache_ids.swap_remove(&key);
        }
    }

    //setup the old and new information so we know what to remove and add for.
    let mut old_players = IndexSet::with_capacity(32);
    let mut old_npcs = IndexSet::with_capacity(32);
    let mut old_items = IndexSet::with_capacity(32);

    let mut new_players = IndexSet::with_capacity(32);
    let mut new_npcs = IndexSet::with_capacity(32);
    let mut new_items = IndexSet::with_capacity(32);

    //create the data tasks to be ran against.
    let mut task_player = Vec::with_capacity(50);
    let mut task_npc = Vec::with_capacity(50);
    let mut task_item = Vec::with_capacity(50);

    //get the old map npcs, players and items so we can send remove requests.
    if let Some(old_map) = oldmap {
        for m in get_surrounding(old_map, true) {
            if let Some(map) = storage.maps.get(&m) {
                map.borrow().players.iter().for_each(|id| {
                    old_players.insert(*id);
                });

                map.borrow().npcs.iter().for_each(|id| {
                    old_npcs.insert(*id);
                });

                map.borrow().itemids.iter().for_each(|id| {
                    old_items.insert(*id);
                });
            }
        }
    }

    //Only get the New id's not in Old for the Vec we use the old data to deturmine what use to exist.
    //the users map is always first in the Vec of get_surrounding so it always gets loaded first.
    for m in get_surrounding(world.get_or_panic::<Position>(user).map, true) {
        if let Some(mapref) = storage.maps.get(&m) {
            let map = mapref.borrow();
            map.players.iter().for_each(|id| {
                if !old_players.contains(id) {
                    task_player.push(*id);
                }

                new_players.insert(*id);
            });

            map.npcs.iter().for_each(|id| {
                if !old_npcs.contains(id) {
                    task_npc.push(*id);
                }

                new_npcs.insert(*id);
            });

            map.itemids.iter().for_each(|id| {
                if !old_items.contains(id) {
                    task_item.push(*id);
                }

                new_items.insert(*id);
            });
        }
    }

    //Gather our Entities to Send for Removal. Type doesnt matter here.
    let mut removals = old_players
        .iter()
        .copied()
        .filter(|id| !new_players.contains(id))
        .collect::<Vec<Entity>>();

    removals.append(
        &mut old_npcs
            .iter()
            .copied()
            .filter(|id| !new_npcs.contains(id))
            .collect::<Vec<Entity>>(),
    );
    removals.append(
        &mut old_items
            .iter()
            .copied()
            .filter(|id| !new_items.contains(id))
            .collect::<Vec<Entity>>(),
    );

    let _ = send_data_remove_list(storage, socket_id, &removals);

    if let Some(tasks) = map_switch_tasks.get_mut(user) {
        tasks.push(MapSwitchTasks::Player(task_player));
        tasks.push(MapSwitchTasks::Npc(task_npc));
        tasks.push(MapSwitchTasks::Items(task_item));
    }
}

const PROCESS_LIMIT: usize = 50;

pub fn process_data_lists(world: &mut World, storage: &Storage) {
    let mut removals = Vec::new();
    let mut maptasks = storage.map_switch_tasks.borrow_mut();

    for (entity, tasks) in maptasks.iter_mut() {
        let mut contains_data = false;

        let socket_id = world.get::<&Socket>(entity.0).map(|s| s.id);
        if let Ok(socket_id) = socket_id {
            for task in tasks {
                let mut count = 0;

                let amount_left = match task {
                    MapSwitchTasks::Npc(entities) => {
                        while let Some(entity) = entities.pop() {
                            let _ = DataTaskToken::NpcSpawnToEntity(socket_id)
                                .add_task(storage, &NpcSpawnPacket::new(world, &entity, false));

                            count += 1;

                            if count >= PROCESS_LIMIT {
                                break;
                            }
                        }

                        entities.len()
                    }
                    MapSwitchTasks::Player(entities) => {
                        while let Some(entity) = entities.pop() {
                            let _ = DataTaskToken::PlayerSpawnToEntity(socket_id)
                                .add_task(storage, &PlayerSpawnPacket::new(world, &entity, false));

                            count += 1;

                            if count >= PROCESS_LIMIT {
                                break;
                            }
                        }

                        entities.len()
                    }
                    MapSwitchTasks::Items(entities) => {
                        while let Some(entity) = entities.pop() {
                            if let Ok(map_item) = world.get::<&MapItem>(entity.0) {
                                let _ = DataTaskToken::ItemLoadToEntity(socket_id).add_task(
                                    storage,
                                    &MapItemPacket::new(
                                        entity,
                                        map_item.pos,
                                        map_item.item,
                                        map_item.ownerid,
                                        false,
                                    ),
                                );

                                count += 1;

                                if count >= PROCESS_LIMIT {
                                    break;
                                }
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
}
