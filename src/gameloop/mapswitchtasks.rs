use crate::{
    containers::{HashSet, Storage},
    gameloop::*,
    gametypes::MapPosition,
    maps::*,
    players::*,
};

/* Information Packet Data Portion Worse case is 1420 bytes
* This means you can fit based on Quantity + 4 byte token header
* Item Size of 17 bytes can send up to 82 per packet.
* Npc Size 80 bytes can send up to 16 per packet.
* player Size 226 bytes can send up to 5 per packet.
*/

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapSwitchTask {
    ownerid: usize,
    mapid: MapPosition,
    currentids: Vec<u64>,
}

impl MapSwitchTask {
    pub fn new(ownerid: usize, mapid: MapPosition) -> MapSwitchTask {
        MapSwitchTask {
            ownerid,
            mapid,
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

pub fn init_data_lists(world: &Storage, user: &mut Player, oldmap: MapPosition) {
    //Remove old tasks and replace with new ones during map switching.
    while let Some(i) = user.map_switch_tasks.pop() {
        world.map_switch_tasks.borrow_mut().remove(i);
    }

    //setup the old and new information so we know what to remove and add for.
    let mut old_players = (
        Vec::<u64>::with_capacity(32),
        HashSet::<u64>::with_capacity_and_hasher(32, Default::default()),
    );
    let mut old_npcs = (
        Vec::<u64>::with_capacity(32),
        HashSet::<u64>::with_capacity_and_hasher(32, Default::default()),
    );
    let mut old_items = (
        Vec::<u64>::with_capacity(32),
        HashSet::<u64>::with_capacity_and_hasher(32, Default::default()),
    );
    let mut new_players = HashSet::<u64>::with_capacity_and_hasher(32, Default::default());
    let mut new_npcs = HashSet::<u64>::with_capacity_and_hasher(32, Default::default());
    let mut new_items = HashSet::<u64>::with_capacity_and_hasher(32, Default::default());

    //create the data tasks to be ran against.
    let mut task_player = MapSwitchTask::new(user.e.get_id(), user.e.pos.map);
    let mut task_npc = MapSwitchTask::new(user.e.get_id(), user.e.pos.map);
    let mut task_item = MapSwitchTask::new(user.e.get_id(), user.e.pos.map);

    //get the old map npcs, players and items so we can send remove requests.
    for m in get_surrounding(oldmap, true) {
        if let Some(map) = world.map_data.get(&m) {
            for id in &map.borrow().players {
                old_players.0.push(*id as u64);
                old_players.1.insert(*id as u64);
            }

            for id in &map.borrow().npcs {
                old_npcs.0.push(*id as u64);
                old_npcs.1.insert(*id as u64);
            }

            for id in &map.borrow().itemids {
                old_items.0.push(*id as u64);
                old_items.1.insert(*id as u64);
            }
        }
    }

    if let Some(map) = world.map_data.get(&user.e.pos.map) {
        //Only get the New id's not in Old for the Vec we use the old data to deturmine what use to exist.
        //This gets them for the main map the rest we will cycle thru.
        //We do this to get the main maps data first.
        for id in &map.borrow().players {
            if !old_players.1.contains(&(*id as u64)) {
                task_player.currentids.push(*id as u64);
            }

            new_players.insert(*id as u64);
        }

        for id in &map.borrow().npcs {
            if !old_npcs.1.contains(&(*id as u64)) {
                task_npc.currentids.push(*id as u64);
            }

            new_npcs.insert(*id as u64);
        }

        for id in &map.borrow().itemids {
            if !old_items.1.contains(&(*id as u64)) {
                task_item.currentids.push(*id as u64);
            }

            new_items.insert(*id as u64);
        }

        //Then we get the rest of the maps so it sends and loads last.
        for m in get_surrounding(user.e.pos.map, true) {
            if m != user.e.pos.map {
                if let Some(map) = world.map_data.get(&m) {
                    for id in &map.borrow().players {
                        if !old_players.1.contains(&(*id as u64)) {
                            task_player.currentids.push(*id as u64);
                        }
                        new_players.insert(*id as u64);
                    }
                    for id in &map.borrow().npcs {
                        if !old_npcs.1.contains(&(*id as u64)) {
                            task_npc.currentids.push(*id as u64);
                        }
                        new_npcs.insert(*id as u64);
                    }
                    for id in &map.borrow().itemids {
                        if !old_items.1.contains(&(*id as u64)) {
                            task_item.currentids.push(*id as u64);
                        }
                        new_items.insert(*id as u64);
                    }
                }
            }
        }
    }

    let _ = send_data_remove_list(
        world,
        user.e.get_id(),
        &old_players
            .0
            .iter()
            .copied()
            .filter(|id| !new_players.contains(id))
            .collect::<Vec<u64>>(),
        1,
    );

    let _ = send_data_remove_list(
        world,
        user.e.get_id(),
        &old_npcs
            .0
            .iter()
            .copied()
            .filter(|id| !new_npcs.contains(id))
            .collect::<Vec<u64>>(),
        0,
    );
    let _ = send_data_remove_list(
        world,
        user.e.get_id(),
        &old_items
            .0
            .iter()
            .copied()
            .filter(|id| !new_items.contains(id))
            .collect::<Vec<u64>>(),
        3,
    );

    user.map_switch_tasks.push(
        world
            .map_switch_tasks
            .borrow_mut()
            .insert(MapSwitchTasks::Player(task_player)),
    );
    user.map_switch_tasks.push(
        world
            .map_switch_tasks
            .borrow_mut()
            .insert(MapSwitchTasks::Npc(task_npc)),
    );
    user.map_switch_tasks.push(
        world
            .map_switch_tasks
            .borrow_mut()
            .insert(MapSwitchTasks::Items(task_item)),
    );
}
