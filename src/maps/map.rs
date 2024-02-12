use crate::{
    containers::{HashSet, IndexSet, Storage},
    gametypes::*,
    maps::MapItem,
};
use bit_op::{bit_u8::*, BitOp};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use unwrap_helpers::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Tile {
    data: [i32; 4],
    attr: u8,
}

//TODO: Update to use MAP (x,y,group) for map locations and Remove map links?
#[derive(Clone, Derivative, Serialize, Deserialize)]
#[derivative(Default(new = "true"))]
pub struct Map {
    pub position: MapPosition,
    #[derivative(Default(value = "[Tile::default(); MAP_MAX_X * MAP_MAX_Y]"))]
    #[serde(with = "BigArray")]
    pub tiles: [Tile; MAP_MAX_X * MAP_MAX_Y],
    // Tiles for zone spawning. (x, y) using u8 to cut down the size and since maps should never Exceed 64x64
    // As super large maps are stupid within a Seamless Structure.
    pub zonespawns: [Vec<(u8, u8)>; 5],
    pub music: u32,
    pub weather: Weather,
    // (Max spawns per zone, [npc_id; 5])
    pub zones: [(u64, [Option<u64>; 5]); 5],
}

impl Map {
    pub fn get_surrounding(&self, include_corners: bool) -> Vec<MapPosition> {
        get_surrounding(self.position, include_corners)
    }
}

#[derive(Clone, Derivative)]
#[derivative(Default(new = "true"))]
pub struct MapData {
    pub position: MapPosition,
    //updated data for map seperate from Map itself as base should be Readonly / clone.
    pub itemids: IndexSet<usize>,
    pub npcs: IndexSet<usize>,
    pub players: IndexSet<usize>,
    #[derivative(Default(value = "slab::Slab::with_capacity(16)"))]
    pub items: slab::Slab<MapItem>,
    pub zones: [u64; 5], //contains the NPC spawn Count of each Zone.
    #[derivative(Default(value = "[(0, false, 0); MAP_MAX_X * MAP_MAX_Y]"))]
    pub move_grid: [(u8, bool, u8); MAP_MAX_X * MAP_MAX_Y], // (count, False=tile|True=Npc or player, Dir Blocking)
    pub players_on_map: u64,
}

impl MapData {
    pub fn get_surrounding(&self, include_corners: bool) -> Vec<MapPosition> {
        get_surrounding(self.position, include_corners)
    }

    #[inline(always)]
    pub fn players_on_map(&self) -> bool {
        self.players_on_map > 0
    }

    pub fn add_mapitem(&mut self, mapitem: MapItem) {
        let id = self.items.insert(mapitem);
        let mut item = self.items.get_mut(id).unwrap();

        item.id = id as u64;
        self.itemids.insert(id);
    }

    pub fn is_blocked_tile(&self, pos: Position) -> bool {
        (self.move_grid[pos.as_tile()].0 > 0 && !self.move_grid[pos.as_tile()].1)
            || (self.move_grid[pos.as_tile()].1 && self.move_grid[pos.as_tile()].0 >= 5)
    }

    pub fn remove_entity_from_grid(&mut self, pos: Position) {
        self.move_grid[pos.as_tile()].0 = self.move_grid[pos.as_tile()].0.saturating_sub(1);

        if self.move_grid[pos.as_tile()].0 == 0 {
            self.move_grid[pos.as_tile()].1 = false;
        }
    }

    pub fn add_entity_to_grid(&mut self, pos: Position) {
        self.move_grid[pos.as_tile()].0 = self.move_grid[pos.as_tile()].0.saturating_add(1);
        self.move_grid[pos.as_tile()].1 = true;
    }

    pub fn add_player(&mut self, world: &mut hecs::World, storage: &Storage, id: usize) {
        self.players.insert(id);

        for i in self.get_surrounding(true) {
            if i != self.position {
                unwrap_continue!(world.maps.get(&i))
                    .borrow_mut()
                    .players_on_map += 1;
            }
        }

        self.players_on_map += 1;
    }

    pub fn add_npc(&mut self, id: usize) {
        self.npcs.insert(id);
    }

    pub fn remove_player(&mut self, world: &mut hecs::World, storage: &Storage, id: usize) {
        self.players.remove(&id);

        for i in self.get_surrounding(true) {
            if i != self.position {
                unwrap_continue!(world.maps.get(&i))
                    .borrow_mut()
                    .players_on_map -= 1;
            }
        }

        self.players_on_map -= 1;
    }

    pub fn remove_npc(&mut self, id: usize) {
        self.npcs.remove(&id);
    }

    pub fn remove_item(&mut self, id: usize) {
        if !self.items.contains(id) {
            return;
        }

        self.items.remove(id);
        self.itemids.remove(&id);
    }
}

pub fn check_surrounding(
    start: MapPosition,
    position: MapPosition,
    include_corners: bool,
) -> MapPos {
    if position == start {
        MapPos::Center(position)
    } else if position == start.map_offset(MapPosDir::Up) {
        MapPos::Up(start.map_offset(MapPosDir::Up))
    } else if position == start.map_offset(MapPosDir::Down) {
        MapPos::Down(start.map_offset(MapPosDir::Down))
    } else if position == start.map_offset(MapPosDir::Left) {
        MapPos::Left(start.map_offset(MapPosDir::Left))
    } else if position == start.map_offset(MapPosDir::Right) {
        MapPos::Right(start.map_offset(MapPosDir::Right))
    } else if include_corners {
        if position == start.map_offset(MapPosDir::UpLeft) {
            MapPos::UpLeft(start.map_offset(MapPosDir::UpLeft))
        } else if position == start.map_offset(MapPosDir::UpRight) {
            MapPos::UpRight(start.map_offset(MapPosDir::UpRight))
        } else if position == start.map_offset(MapPosDir::DownLeft) {
            MapPos::DownLeft(start.map_offset(MapPosDir::DownLeft))
        } else if position == start.map_offset(MapPosDir::DownRight) {
            MapPos::DownRight(start.map_offset(MapPosDir::DownRight))
        } else {
            MapPos::None
        }
    } else {
        MapPos::None
    }
}

pub fn get_dir_mapid(
    world: &Storage,
    position: MapPosition,
    dir: MapPosDir,
) -> Option<MapPosition> {
    let offset = position.map_offset(dir);
    let _ = world.bases.maps.get(&offset)?;
    Some(offset)
}

pub fn get_surrounding(position: MapPosition, include_corners: bool) -> Vec<MapPosition> {
    let mut arr = vec![
        position,
        position.map_offset(MapPosDir::Up),
        position.map_offset(MapPosDir::Down),
        position.map_offset(MapPosDir::Left),
        position.map_offset(MapPosDir::Right),
    ];

    if include_corners {
        arr.push(position.map_offset(MapPosDir::UpLeft));
        arr.push(position.map_offset(MapPosDir::UpRight));
        arr.push(position.map_offset(MapPosDir::DownLeft));
        arr.push(position.map_offset(MapPosDir::DownRight));
    }

    arr
}

pub fn get_surrounding_dir(position: MapPosition, include_corners: bool) -> Vec<MapPos> {
    let mut arr = vec![
        MapPos::Center(position),
        MapPos::Up(position.map_offset(MapPosDir::Up)),
        MapPos::Down(position.map_offset(MapPosDir::Down)),
        MapPos::Left(position.map_offset(MapPosDir::Left)),
        MapPos::Right(position.map_offset(MapPosDir::Right)),
    ];

    if include_corners {
        arr.push(MapPos::UpLeft(position.map_offset(MapPosDir::UpLeft)));
        arr.push(MapPos::UpRight(position.map_offset(MapPosDir::UpRight)));
        arr.push(MapPos::DownLeft(position.map_offset(MapPosDir::DownLeft)));
        arr.push(MapPos::DownRight(position.map_offset(MapPosDir::DownRight)));
    }

    arr
}

pub fn get_extended_surrounding_set(position: MapPosition) -> HashSet<MapPosition> {
    let mut set = HashSet::<MapPosition>::default();

    for next_position in get_surrounding(position, true) {
        let outer_positions = get_surrounding(next_position, true);

        for pos in outer_positions {
            set.insert(pos);
        }

        set.insert(next_position);
    }

    set
}

pub fn get_surrounding_set(position: MapPosition) -> HashSet<MapPosition> {
    let mut set = HashSet::<MapPosition>::default();

    for next_position in get_surrounding(position, true) {
        set.insert(next_position);
    }

    set
}

//Allowed_maps is a limit set so we dont process every map.
//Processed is so we dont redo maps that we already looked into.
//This is a recrusive function gets the End positions Offset
// position based on start position.
pub fn map_offset_range(
    start: Position,
    endpos: Position,
    allowed_maps: &HashSet<MapPosition>,
    processed: &mut HashSet<MapPosition>,
) -> Option<Position> {
    allowed_maps.get(&endpos.map)?;

    if start.map == endpos.map {
        return Some(endpos);
    }

    let dirs = get_surrounding_dir(start.map, false);
    processed.insert(start.map);
    // lets check each surrounding map first to make sure its not here
    // before we span into the other maps.
    for dir in &dirs {
        if dir.contains(endpos.map) {
            return Some(endpos.map_offset(dir.into()));
        }
    }

    //Else if not found above lets start searching within each side part ignoring
    //Maps not within the Allowed HashSet.
    for dir in &dirs {
        let x = unwrap_continue!(dir.get());

        if allowed_maps.get(&x).is_none() || processed.get(&x).is_some() {
            continue;
        }

        let end = endpos.map_offset(dir.into());
        let pos = Position::new(0, 0, x);
        let ret = map_offset_range(pos, end, allowed_maps, processed);

        //if it is Some then we did find it and get the offset so lets return it.
        if ret.is_some() {
            return ret;
        }
    }

    None
}

pub fn get_maps_in_range(world: &Storage, pos: &Position, range: i32) -> Vec<MapPos> {
    let mut arr: Vec<MapPos> = Vec::new();
    unwrap_or_return!(world.bases.maps.get(&pos.map), Vec::new());

    arr.push(MapPos::Center(pos.map));

    if pos.x - range < 0 && pos.y - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::UpLeft);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::UpLeft(pos));
        }
    }

    if pos.x - range < 0 && pos.y + range >= MAP_MAX_Y as i32 {
        let pos = pos.map.map_offset(MapPosDir::DownLeft);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::DownLeft(pos));
        }
    }

    if pos.x + range < 0 && pos.y - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::UpRight);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::UpRight(pos));
        }
    }

    if pos.x + range < 0 && pos.y + range >= MAP_MAX_Y as i32 {
        let pos = pos.map.map_offset(MapPosDir::DownRight);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::DownRight(pos));
        }
    }

    if pos.x - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::Left);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Left(pos));
        }
    }

    if pos.x + range >= MAP_MAX_X as i32 {
        let pos = pos.map.map_offset(MapPosDir::Right);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Right(pos));
        }
    }

    if pos.y - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::Up);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Up(pos));
        }
    }

    if pos.y + range >= MAP_MAX_Y as i32 {
        let pos = pos.map.map_offset(MapPosDir::Down);

        if world.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Down(pos));
        }
    }

    arr
}

pub fn map_path_blocked(
    world: &Storage,
    cur_pos: Position,
    next_pos: Position,
    movedir: u8,
) -> bool {
    let blocked = match movedir {
        0 => {
            if let Some(map) = world.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].2.get(B0) == 0b00000001
            } else {
                true
            }
        }
        1 => {
            if let Some(map) = world.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].2.get(B3) == 0b00001000
            } else {
                true
            }
        }
        2 => {
            if let Some(map) = world.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].2.get(B1) == 0b00000010
            } else {
                true
            }
        }
        _ => {
            if let Some(map) = world.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].2.get(B2) == 0b00000100
            } else {
                true
            }
        }
    };

    if !blocked {
        return unwrap_or_return!(world.maps.get(&next_pos.map), true)
            .borrow()
            .is_blocked_tile(next_pos);
    }

    blocked
}
