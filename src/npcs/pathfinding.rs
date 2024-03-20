use crate::{containers::*, gametypes::*, maps::*};
use rand::{thread_rng, Rng};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, VecDeque},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PathNode {
    g: i32,
    h: i32,
    f: i32,
    parent: Option<usize>,
    pos: Position,
    dir: u8,
    offset: Position,
}

impl PathNode {
    pub fn new(pos: Position, dir: u8, offset: Position, parent: Option<usize>) -> Self {
        Self {
            g: 0,
            h: 0,
            f: 0,
            parent,
            pos,
            dir,
            offset,
        }
    }
}

pub fn path_map_switch(
    storage: &Storage,
    allowed_maps: &HashSet<MapPosition>,
    cur_pos: Position,
    next_pos: &mut Position,
    movedir: u8,
) -> bool {
    //Checks if the Map Exists within our MapDir and if so sets and returns true if it was successful
    //or false if we can not switch the map.
    let set_pos = |next_pos: &mut Position, mappos, x, y| -> bool {
        let mapid = match get_dir_mapid(storage, next_pos.map, mappos) {
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
            0 => return set_pos(next_pos, MapPosDir::Up, cur_pos.x, 0),
            1 => return set_pos(next_pos, MapPosDir::Right, 0, cur_pos.y),
            2 => return set_pos(next_pos, MapPosDir::Down, cur_pos.x, MAP_MAX_Y as i32 - 1),
            _ => return set_pos(next_pos, MapPosDir::Left, MAP_MAX_X as i32 - 1, cur_pos.y),
        }
    }

    true
}

pub fn a_star_path(
    storage: &Storage,
    start: Position,
    dir: u8,
    stop: Position,
) -> Option<VecDeque<(Position, u8)>> {
    let mut id = 0;
    let mut nodes = Vec::with_capacity(32);
    let mut opened = BinaryHeap::with_capacity(32);
    let mut closed = HashMap::with_capacity_and_hasher(32, Default::default());
    //Build the list of allowed maps the npc can look at when building its path. We want to limit this so they can not kill the
    //server by over processing. If the target is outside these maps we should untarget them.
    let allowed_maps = get_extended_surrounding_set(start.map);
    //down, right, up, left
    let adjacent = [(0, -1), (1, 0), (0, 1), (-1, 0)];

    //see if the map ID exists in our limit searc range of maps
    allowed_maps.get(&stop.map)?;

    //find the Offset position of the end position for Calculations
    let stop_offset = match map_offset_range(start, stop, &allowed_maps, &mut HashSet::default()) {
        Some(pos) => pos,
        None => return None,
    };

    nodes.push(PathNode::new(start, dir, start, None));
    opened.push(Reverse((0, id)));
    id += 1;

    while let Some(Reverse((_, current_index))) = opened.pop() {
        let current_node = nodes[current_index];

        closed.insert(nodes[current_index].pos, current_index);

        //If we hit the Final location we then go backwards from the current node
        //And follow the Parents till the Start location.
        if current_node.pos == stop {
            let mut path = VecDeque::with_capacity(16);
            let mut current = current_node;

            loop {
                //Push it so the Back ends up in the back and the first moves are in the front.
                path.push_front((current.pos, current.dir));

                if let Some(nextid) = current.parent {
                    //Prevent us from using the position our Aggressor is on.
                    if nodes[nextid].pos != start {
                        current = nodes[nextid];
                    } else {
                        return Some(path);
                    }
                } else {
                    return Some(path);
                }
            }
        }

        let mut children_nodes = Vec::with_capacity(16);

        //Cycle each direction to get a Rated path ontop of each current location.
        //We only build out and process 4 directions at a time. this helps us
        //to reduce the need to build the entire tile map.
        for (i, movedir) in adjacent.iter().enumerate() {
            // We create a node that gets its position updated base on maps.
            let mut node_pos = Position::new(
                current_node.pos.x + movedir.0,
                current_node.pos.y + movedir.1,
                current_node.pos.map,
            );

            //This is the actual node movement made so we know the ID later
            let node_offset = Position::new(
                current_node.offset.x + movedir.0,
                current_node.offset.y + movedir.1,
                current_node.pos.map,
            );

            //Check if it will be a map switch and if so generate the correct location for node_pos.
            if !path_map_switch(
                storage,
                &allowed_maps,
                current_node.pos,
                &mut node_pos,
                i as u8,
            ) {
                continue;
            }

            //ignore any that is the Same location as the start or stop locations.
            if (node_pos.x == start.x && node_pos.y == start.y && node_pos.map == start.map)
                || (node_pos.x == stop.x || node_pos.y == stop.y || node_pos.map == stop.map)
            {
                continue;
            }

            //Make sure the node is not blocked otherwise we wont count it in our node list.
            if map_path_blocked(storage, current_node.pos, node_pos, i as u8) {
                continue;
            }

            //Place into a Vec to process them out in the next step if they pass the initial criteria.
            children_nodes.push(PathNode::new(
                node_pos,
                i as u8,
                node_offset,
                Some(current_index),
            ));
        }

        //We now cycle through the Children we gathered.
        'found: for mut child in children_nodes {
            //we make sure we are not heading back towards tiles we already processed once.
            if closed.contains_key(&child.pos) {
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

            for Reverse((_, opened_id)) in &opened {
                // if we loop in here and one of our last pushed Nodes == one of our new children
                // we will ignore it and continue with more children as it already exists.
                //TODO: change this to a HashSet to avoid looping everything and get a much faster lookup.
                if nodes[*opened_id].pos == child.pos && child.g >= nodes[*opened_id].g {
                    continue 'found;
                }
            }

            //If the node was worthy we then push it to the end of our list.
            nodes.push(child);
            //We then push it to our open node list of nodes we want to pop off. Best node gets popped off first based on its F value.
            opened.push(Reverse((child.f, id)));
            id += 1;
        }
    }

    //We ran out of Tiles to check and crashed out failed to find path to target.
    None
}

pub fn npc_rand_movement(storage: &Storage, pos: Position, dir: u8) -> VecDeque<(Position, u8)> {
    let mut rng = thread_rng();
    //down, right, up, left
    let adjacent = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut path = VecDeque::with_capacity(16);
    let mut lastpos = pos;
    let mut lastdir: usize = dir as usize;
    let allowed_maps = get_surrounding_set(pos.map);

    //Lets get a range of movements in one go.
    for _ in 1..rng.gen_range(3..10) {
        let movedir = rng.gen_range(0..=3);
        let mut node_pos = Position::new(
            lastpos.x + adjacent[movedir].0,
            lastpos.y + adjacent[movedir].1,
            lastpos.map,
        );

        if node_pos.x < 0 || node_pos.x >= 32 || node_pos.y < 0 || node_pos.y >= 32 {
            let adj = [
                (lastpos.x, 31),
                (0, lastpos.y),
                (lastpos.x, 0),
                (31, lastpos.y),
            ];

            node_pos = Position::new(
                adj[movedir as usize].0,
                adj[movedir as usize].1,
                MapPosition {
                    x: lastpos.map.x + adjacent[movedir as usize].0,
                    y: lastpos.map.y + adjacent[movedir as usize].1,
                    group: lastpos.map.group,
                },
            );
        }

        if !path_map_switch(
            storage,
            &allowed_maps,
            lastpos,
            &mut node_pos,
            movedir as u8,
        ) {
            continue;
        }

        if map_path_blocked(storage, lastpos, node_pos, movedir as u8) && lastdir != movedir {
            path.push_back((lastpos, movedir as u8));
        } else {
            path.push_back((node_pos, movedir as u8));
            lastpos = node_pos;
        }
        lastdir = movedir;
    }

    path
}
