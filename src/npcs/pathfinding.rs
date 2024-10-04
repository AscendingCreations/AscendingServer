use crate::{gametypes::*, maps::*};
use core::cell::RefCell;
use rand::{thread_rng, Rng};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
};

thread_local! {
    static NODES: RefCell<Vec<PathNode>> = RefCell::new(Vec::with_capacity(256));
    static OPENED: RefCell<BinaryHeap<Reverse<(i32, usize)>>> = RefCell::new(BinaryHeap::with_capacity(256));
    static CLOSED: RefCell<HashMap<Position, usize>> = RefCell::new(HashMap::with_capacity_and_hasher(256, ahash::RandomState::new()));
    static CONTAINED: RefCell<HashMap<Position, usize>> = RefCell::new(HashMap::with_capacity_and_hasher(256, ahash::RandomState::new()));
    static CHILDREN_NODES: RefCell<Vec<PathNode>>  = RefCell::new(Vec::with_capacity(16));
}

pub fn path_map_switch(
    map: &MapActor,
    allowed_maps: &HashSet<MapPosition>,
    cur_pos: Position,
    next_pos: &mut Position,
    movedir: Dir,
) -> bool {
    //Checks if the Map Exists within our MapDir and if so sets and returns true if it was successful
    //or false if we can not switch the map.
    let set_pos = |next_pos: &mut Position, mappos, x, y| -> bool {
        let mapid = match map.get_dir_mapid(next_pos.map, mappos) {
            Some(id) => id,
            None => return false,
        };

        *next_pos = match allowed_maps.get(&mapid) {
            Some(_) => Position::new(x, y, mapid),
            None => return false,
        };

        true
    };

    if next_pos.left_map() {
        match movedir {
            Dir::Down => set_pos(next_pos, MapDir::Down, cur_pos.x, MAP_MAX_Y as i32 - 1),
            Dir::Right => set_pos(next_pos, MapDir::Right, 0, cur_pos.y),
            Dir::Up => set_pos(next_pos, MapDir::Up, cur_pos.x, 0),
            Dir::Left => set_pos(next_pos, MapDir::Left, MAP_MAX_X as i32 - 1, cur_pos.y),
        }
    } else {
        true
    }
}

pub fn a_star_path(
    map: &mut MapActor,
    start: Position,
    dir: Dir,
    stop: Position,
) -> Option<VecDeque<(Position, Dir)>> {
    let mut id = 0;
    //Build the list of allowed maps the npc can look at when building its path. We want to limit this so they can not kill the
    //server by over processing. If the target is outside these maps we should untarget them.
    let allowed_maps = get_surrounding_set(start.map);

    //see if the map ID exists in our limit searc range of maps
    allowed_maps.get(&stop.map)?;

    NODES.with_borrow_mut(|n| n.clear());
    OPENED.with_borrow_mut(|n| n.clear());
    CHILDREN_NODES.with_borrow_mut(|n| n.clear());
    CLOSED.with_borrow_mut(|n| n.clear());
    CONTAINED.with_borrow_mut(|n| n.clear());

    //find the Offset position of the end position for Calculations
    let stop_offset = match map_offset_range(start, stop, &allowed_maps, &mut HashSet::default()) {
        Some(pos) => pos,
        None => return None,
    };

    NODES.with_borrow_mut(|n| n.push(PathNode::new(start, dir, start, None)));
    OPENED.with_borrow_mut(|n| n.push(Reverse((0, id))));
    id += 1;

    while let Some(Reverse((_, current_index))) = OPENED.with_borrow_mut(|n| n.pop()) {
        let current_node = NODES.with_borrow(|n| n[current_index]);

        CLOSED.with_borrow_mut(|n| {
            n.insert(NODES.with_borrow(|n| n[current_index].pos), current_index)
        });

        //If we hit the Final location we then go backwards from the current node
        //And follow the Parents till the Start location. if the final location is blocked we will
        //find a location within a range of 1 regardless if blocked or not.
        if map.is_blocked_tile(stop, WorldEntityType::Npc)
            && map.in_dir_attack_zone(current_node.pos, stop, 1)
        {
            return npc_path_gather(&current_node, start);
        }

        if current_node.pos == stop {
            return npc_path_gather(&current_node, start);
        }

        //Cycle each direction to get a Rated path ontop of each current location.
        //We only build out and process 4 directions at a time. this helps us
        //to reduce the need to build the entire tile map.
        for dir in Dir::all() {
            let (x, y) = dir.xy_offset();
            // We create a node that gets its position updated base on maps.
            let mut node_pos = Position::new(
                current_node.pos.x + x,
                current_node.pos.y + y,
                current_node.pos.map,
            );

            //This is the actual node movement made so we know the ID later
            let node_offset = Position::new(
                current_node.offset.x + x,
                current_node.offset.y + y,
                current_node.pos.map,
            );

            //Check if it will be a map switch and if so generate the correct location for node_pos.
            if !path_map_switch(map, &allowed_maps, current_node.pos, &mut node_pos, dir) {
                continue;
            }

            //ignore any that is the Same location as the start location.
            if node_pos.x == start.x && node_pos.y == start.y && node_pos.map == start.map {
                continue;
            }

            //Make sure the node is not blocked otherwise we wont count it in our node list.
            if map.map_path_blocked(current_node.pos, node_pos, dir, WorldEntityType::Npc) {
                continue;
            }

            //Place into a Vec to process them out in the next step if they pass the initial criteria.
            CHILDREN_NODES.with_borrow_mut(|n| {
                n.push(PathNode::new(
                    node_pos,
                    dir,
                    node_offset,
                    Some(current_index),
                ))
            });
        }

        //We now cycle through the Children we gathered.
        'found: while let Some(mut child) = CHILDREN_NODES.with_borrow_mut(|n| n.pop()) {
            //we make sure we are not heading back towards tiles we already processed once.
            if CLOSED.with_borrow(|n| n.contains_key(&child.pos)) {
                continue 'found;
            }

            //incremental to know we are moving in a direction to increase the Value so they get more expensive
            //the further we move away from our original position
            child.g = current_node.g + 1;
            //Distance from the end. this value helps deturmine if a Direction is outside of our scope or if its the shortest path.
            child.h =
                (child.offset.x - stop_offset.x).pow(2) + (child.offset.y - stop_offset.y).pow(2);
            //Add them together so we get the most accurate Value to use to deturmine which Tile is best to move too
            //out of the children we processed.
            child.f = child.g + child.h;

            if let Some(i) = CONTAINED.with_borrow(|n| n.get(&child.pos).copied()) {
                let node: PathNode = NODES.with_borrow(|n| n[i]);
                if child.g >= node.g {
                    continue 'found;
                }
            }

            //should allow for quicker find than looping backwards.
            CONTAINED.with_borrow_mut(|n| n.insert(child.pos, id));
            //If the node was worthy we then push it to the end of our list.
            NODES.with_borrow_mut(|n| n.push(child));
            //We then push it to our open node list of nodes we want to pop off. Best node gets popped off first based on its F value.
            OPENED.with_borrow_mut(|n| n.push(Reverse((child.f, id))));
            id += 1;
        }

        CHILDREN_NODES.with_borrow_mut(|n| n.clear());
    }

    //We ran out of Tiles to check and crashed out failed to find path to target.
    None
}

pub fn npc_path_gather(
    current_node: &PathNode,
    start: Position,
) -> Option<VecDeque<(Position, Dir)>> {
    let mut path = VecDeque::with_capacity(64);
    let mut current = *current_node;

    while let Some(nextid) = current.parent {
        if NODES.with_borrow(|n| n[nextid].pos) != start {
            current = NODES.with_borrow(|n| n[nextid]);
            path.push_front((current.pos, current.dir));
        } else {
            break;
        }
    }

    Some(path)
}

pub fn npc_rand_movement(map: &MapActor, pos: Position) -> VecDeque<(Position, Dir)> {
    let mut rng = thread_rng();
    let mut path = VecDeque::with_capacity(16);
    let mut lastpos = pos;
    let allowed_maps = get_surrounding_set(pos.map);

    //Lets get a range of movements in one go.
    for _ in 1..rng.gen_range(3..10) {
        let movedir = Dir::from(rng.gen_range(0..=3));
        let (x, y) = movedir.xy_offset();
        let mut node_pos = Position::new(lastpos.x + x, lastpos.y + y, lastpos.map);

        if !path_map_switch(map, &allowed_maps, lastpos, &mut node_pos, movedir) {
            continue;
        }

        if map.map_path_blocked(lastpos, node_pos, movedir, WorldEntityType::Npc) {
            path.push_back((lastpos, movedir));
        } else {
            path.push_back((node_pos, movedir));
            lastpos = node_pos;
        }
    }

    path
}
