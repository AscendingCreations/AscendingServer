use crate::{
    containers::{HashSet, Storage},
    gameloop::*,
    gametypes::MapPosition,
    maps::*,
    players::*,
    gametypes::*,
};

/* Information Packet Data Portion Worse case is 1420 bytes
* This means you can fit based on Quantity + 4 byte token header + 4 bytes for count
* Item Size of 17 bytes can send up to 82 per packet.
* Npc Size 80 bytes can send up to 16 per packet.
* player Size 226 bytes can send up to 5 per packet.
*/

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapSwitchTask {
    ownerid: Entity,
    currentids: Vec<Entity>,
}

impl MapSwitchTask {
    pub fn new(ownerid: Entity) -> MapSwitchTask {
        MapSwitchTask {
            ownerid,
            currentids: Vec::with_capacity(32),
        }
    }
}

//types to buffer load when loading a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapSwitchTasks {
    Npc(MapSwitchTask),    //0
    Player(MapSwitchTask), //1
    Items(MapSwitchTask),  //2
}

pub fn init_data_lists(world: &hecs::World, storage: &Storage, user: &crate::Entity, oldmap: MapPosition) {
    //Remove old tasks and replace with new ones during map switching.
    while let Some(i) = 
        world.get::<&mut crate::players::MapSwitchTasks>(user.0).expect("Could not find MapSwitchTasks").tasks.pop() {
        storage.map_switch_tasks.borrow_mut().remove(i);
    }

    //setup the old and new information so we know what to remove and add for.
    let mut old_players = (
        Vec::<crate::Entity>::with_capacity(32),
        HashSet::<crate::Entity>::with_capacity_and_hasher(32, Default::default()),
    );
    let mut old_npcs = (
        Vec::<crate::Entity>::with_capacity(32),
        HashSet::<crate::Entity>::with_capacity_and_hasher(32, Default::default()),
    );
    let mut old_items = (
        Vec::<crate::Entity>::with_capacity(32),
        HashSet::<crate::Entity>::with_capacity_and_hasher(32, Default::default()),
    );
    let mut new_players = HashSet::<crate::Entity>::with_capacity_and_hasher(32, Default::default());
    let mut new_npcs = HashSet::<crate::Entity>::with_capacity_and_hasher(32, Default::default());
    let mut new_items = HashSet::<crate::Entity>::with_capacity_and_hasher(32, Default::default());

    //create the data tasks to be ran against.
    let mut task_player = MapSwitchTask::new(*user);
    let mut task_npc = MapSwitchTask::new(*user);
    let mut task_item = MapSwitchTask::new(*user);

    //get the old map npcs, players and items so we can send remove requests.
    for m in get_surrounding(oldmap, true) {
        if let Some(map) = storage.maps.get(&m) {
            for id in &map.borrow().players {
                old_players.0.push(*id);
                old_players.1.insert(*id);
            }

            for id in &map.borrow().npcs {
                old_npcs.0.push(*id);
                old_npcs.1.insert(*id);
            }

            for id in &map.borrow().itemids {
                old_items.0.push(*id);
                old_items.1.insert(*id); 
            }
        }
    }

    if let Some(map) = 
        storage.maps
            .get(&world.get_or_panic::<Position>(user).map) {
        //Only get the New id's not in Old for the Vec we use the old data to deturmine what use to exist.
        //This gets them for the main map the rest we will cycle thru.
        //We do this to get the main maps data first.
        for id in &map.borrow().players {
            if !old_players.1.contains(&(*id)) {
                task_player.currentids.push(*id);
            }

            new_players.insert(*id);
        }

        for id in &map.borrow().npcs {
            if !old_npcs.1.contains(&(*id)) {
                task_npc.currentids.push(*id);
            }

            new_npcs.insert(*id);
        }

        for id in &map.borrow().itemids {
            if !old_items.1.contains(&(*id)) {
                task_item.currentids.push(*id);
            }

            new_items.insert(*id);
        }

        //Then we get the rest of the maps so it sends and loads last.
        for m in 
            get_surrounding(world.get_or_panic::<Position>(user).map, true) {
            if m != world.get_or_panic::<Position>(user).map {
                if let Some(map) = storage.maps.get(&m) {
                    for id in &map.borrow().players {
                        if !old_players.1.contains(&(*id)) {
                            task_player.currentids.push(*id);
                        }
                        new_players.insert(*id);
                    }
                    for id in &map.borrow().npcs {
                        if !old_npcs.1.contains(&(*id)) {
                            task_npc.currentids.push(*id);
                        }
                        new_npcs.insert(*id);
                    }
                    for id in &map.borrow().itemids {
                        if !old_items.1.contains(&(*id)) {
                            task_item.currentids.push(*id);
                        }
                        new_items.insert(*id);
                    }
                }
            }
        }
    }

    let _ = send_data_remove_list(
        storage,
        world.get_or_panic::<&Socket>(user).id,
        &old_players
            .0
            .iter()
            .copied()
            .filter(|id| !new_players.contains(id))
            .collect::<Vec<Entity>>(),
        1,
    );

    let _ = send_data_remove_list(
        storage,
        world.get_or_panic::<&Socket>(user).id,
        &old_npcs
            .0
            .iter()
            .copied()
            .filter(|id| !new_npcs.contains(id))
            .collect::<Vec<Entity>>(),
        0,
    );
    let _ = send_data_remove_list(
        storage,
        world.get_or_panic::<&Socket>(user).id,
        &old_items
            .0
            .iter()
            .copied()
            .filter(|id| !new_items.contains(id))
            .collect::<Vec<Entity>>(),
        3,
    );

    world.get::<&mut crate::players::MapSwitchTasks>(user.0).expect("Could not find MapSwitchTasks")
        .tasks.push(
            storage
                .map_switch_tasks
                .borrow_mut()
                .insert(MapSwitchTasks::Player(task_player)),
        );
    world.get::<&mut crate::players::MapSwitchTasks>(user.0).expect("Could not find MapSwitchTasks")
        .tasks.push(
            storage
                .map_switch_tasks
                .borrow_mut()
                .insert(MapSwitchTasks::Player(task_npc)),
        );
    world.get::<&mut crate::players::MapSwitchTasks>(user.0).expect("Could not find MapSwitchTasks")
        .tasks.push(
            storage
                .map_switch_tasks
                .borrow_mut()
                .insert(MapSwitchTasks::Player(task_item)),
        );
}
