use crate::{containers::*, gametypes::*, maps::*};
use rand::{thread_rng, Rng};
use std::{cmp::Reverse, collections::{BinaryHeap, VecDeque}};

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
            0 => return set_pos(next_pos, MapPosDir::Up, cur_pos.x, MAP_MAX_Y as i32 - 1),
            1 => return set_pos(next_pos, MapPosDir::Right, 0, cur_pos.y),
            2 => return set_pos(next_pos, MapPosDir::Down, cur_pos.x, 0),
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
    let allowed_maps = get_extended_surrounding_set(start.map);
    //up, down, left, right
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

        if current_node.pos == stop {
            let mut path = VecDeque::with_capacity(16);
            let mut current = current_node;

            loop {
                path.push_front((current.pos, current.dir));

                if let Some(nextid) = current.parent {
                    current = nodes[nextid]
                } else {
                    return Some(path);
                }
            }
        }

        let mut children_nodes = Vec::with_capacity(16);

        for (i, movedir) in adjacent.iter().enumerate() {
            let mut node_pos = Position::new(
                current_node.pos.x + movedir.0,
                current_node.pos.y + movedir.1,
                current_node.pos.map,
            );
            let node_offset = Position::new(
                current_node.offset.x + movedir.0,
                current_node.offset.y + movedir.1,
                current_node.pos.map,
            );

            if !path_map_switch(
                storage,
                &allowed_maps,
                current_node.pos,
                &mut node_pos,
                i as u8,
            ) {
                continue;
            }

            if map_path_blocked(storage, current_node.pos, node_pos, i as u8)
                && (node_pos.x != start.x || node_pos.y != start.y || node_pos.map != start.map)
                && (node_pos.x != stop.x || node_pos.y != stop.y || node_pos.map != stop.map)
            {
                continue;
            }

            children_nodes.push(PathNode::new(
                node_pos,
                i as u8,
                node_offset,
                Some(current_index),
            ));
        }

        'found: for mut child in children_nodes {
            if closed.contains_key(&child.pos) {
                continue 'found;
            }

            child.g = current_node.g + 1;
            child.h =
                (child.offset.x - stop_offset.x).pow(2) + (child.offset.y - stop_offset.y).pow(2);
            child.f = child.g + child.h;

            for Reverse((_, opened_id)) in &opened {
                if nodes[*opened_id].pos == child.pos && child.g >= nodes[*opened_id].g {
                    continue 'found;
                }
            }

            nodes.push(child);
            opened.push(Reverse((child.f, id)));
            id += 1;
        }
    }

    None
}

pub fn npc_rand_movement(storage: &Storage, pos: Position, dir: u8) -> VecDeque<(Position, u8)> {
    let mut rng = thread_rng();
    let adjacent = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut path = VecDeque::with_capacity(16);
    let mut lastpos = pos;
    let mut lastdir: usize = dir as usize;
    //let allowed_maps = get_surrounding_set(pos.map);

    for _ in 1..rng.gen_range(2..10) {
        let movedir = rng.gen_range(0..=3);
        let mut node_pos = Position::new(
            lastpos.x + adjacent[movedir].0,
            lastpos.y + adjacent[movedir].1,
            lastpos.map,
        );

        if node_pos.x < 0 || node_pos.x >= 32 || node_pos.y < 0 || node_pos.y >= 32 {
            let adj = [(lastpos.x, 31), (0, lastpos.y), (lastpos.x, 0), (31, lastpos.y)];
            let map_adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
            node_pos = Position::new(
                adj[movedir as usize].0,
                adj[movedir as usize].1,
                MapPosition {
                    x: lastpos.map.x + map_adj[movedir as usize].0,
                    y: lastpos.map.y + map_adj[movedir as usize].1,
                    group: lastpos.map.group
                }
            );
        }

        /*if !path_map_switch(
            storage,
            &allowed_maps,
            lastpos,
            &mut node_pos,
            movedir as u8,
        ) {
            continue;
        }*/

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
