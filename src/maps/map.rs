use crate::{
    containers::{HashSet, IndexSet, Storage},
    gametypes::*,
    maps::MapItem,
    time_ext::MyInstant,
};
use bit_op::{bit_u8::*, BitOp};
use hecs::World;
use serde::{Deserialize, Serialize};
//use serde_big_array::BigArray;

use std::fs::{self, OpenOptions};
use std::io::BufReader;
use std::path::Path;

const MAP_PATH: &str = "./data/maps/";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WarpData {
    pub map_x: i32,
    pub map_y: i32,
    pub map_group: u64,
    pub tile_x: u32,
    pub tile_y: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ItemSpawnData {
    pub index: u32,
    pub amount: u16,
    pub timer: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum MapAttribute {
    #[default]
    Walkable,
    Blocked,
    Warp(WarpData),
    Sign(String),
    ItemSpawn(ItemSpawnData),
    Count,
}

/// The Block Type per Tile. This does not include Attributes so you will need
/// to cycle through the Static Map Attribute Vec to get that information.
/// this is only used to deturmine if something is blocked or not.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GridAttribute {
    #[default]
    Walkable,
    Entity,
    Blocked,
    NpcBlock,
}

/// Data that is changable per Tile for Blocking purposes within MapData
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GridTile {
    pub count: u8,
    pub attr: GridAttribute,
    pub dir_block: u8,
}

//TODO: Update to use MAP (x,y,group) for map locations and Remove map links?
#[derive(Clone, Derivative, Serialize, Deserialize)]
#[derivative(Default(new = "true"))]
pub struct Map {
    pub position: MapPosition,
    //#[derivative(Default(value = "[Tile::default(); MAP_MAX_X * MAP_MAX_Y]"))]
    //#[serde(with = "BigArray")]
    //pub tiles: [Tile; MAP_MAX_X * MAP_MAX_Y],
    pub attribute: Vec<MapAttribute>,
    // Tiles for zone spawning. (x, y) using u8 to cut down the size and since maps should never Exceed 64x64
    // As super large maps are stupid within a Seamless Structure.
    pub zonespawns: [Vec<(u16, u16)>; 5],
    // (Max spawns per zone, [npc_id; 5])
    pub zones: [(u64, [Option<u64>; 5]); 5],
    pub music: u32,
    pub weather: Weather,
}

impl Map {
    pub fn get_surrounding(&self, include_corners: bool) -> Vec<MapPosition> {
        get_surrounding(self.position, include_corners)
    }
}

pub fn get_maps() -> Vec<Map> {
    let entries = fs::read_dir(MAP_PATH).unwrap();

    let mut map_data: Vec<Map> = Vec::new();

    for entry_data in entries.flatten() {
        if let Ok(filename) = entry_data.file_name().into_string() {
            if let Some(mapdata) = load_map(filename) {
                map_data.push(mapdata);
            }
        }
    }

    map_data
}

fn load_map(filename: String) -> Option<Map> {
    let name = format!("{}{}", MAP_PATH, filename);

    if !Path::new(&name).exists() {
        println!("Map does not exist");
        return None;
    }

    match OpenOptions::new().read(true).open(&name) {
        Ok(file) => {
            let reader = BufReader::new(file);

            match serde_json::from_reader(reader) {
                Ok(data) => Some(data),
                Err(e) => {
                    println!("Failed to load {}, Err {:?}", name, e);
                    None
                }
            }
        }
        Err(e) => {
            println!("Failed to load {}, Err {:?}", name, e);
            None
        }
    }
}

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct SpawnItemData {
    pub index: u32,
    pub amount: u16,
    pub pos: Position,
    pub timer_set: u64,
    // Editable
    #[derivative(Default(value = "MyInstant::now()"))]
    pub timer: MyInstant,
}

#[derive(Clone, Derivative)]
#[derivative(Default(new = "true"))]
pub struct MapData {
    pub position: MapPosition,
    //updated data for map seperate from Map itself as base should be Readonly / clone.
    pub itemids: IndexSet<Entity>,
    pub npcs: IndexSet<Entity>,
    pub players: IndexSet<Entity>,
    //#[derivative(Default(value = "slab::Slab::with_capacity(16)"))]
    //pub items: slab::Slab<MapItem>,
    pub zones: [u64; 5], //contains the NPC spawn Count of each Zone.
    #[derivative(Default(value = "[GridTile::default(); MAP_MAX_X * MAP_MAX_Y]"))]
    pub move_grid: [GridTile; MAP_MAX_X * MAP_MAX_Y], // (count, False=tile|True=Npc or player, Dir Blocking)
    pub players_on_map: u64,
    pub spawnable_item: Vec<SpawnItemData>,
}

impl MapData {
    pub fn get_surrounding(&self, include_corners: bool) -> Vec<MapPosition> {
        get_surrounding(self.position, include_corners)
    }

    #[inline(always)]
    pub fn players_on_map(&self) -> bool {
        self.players_on_map > 0
    }

    pub fn add_spawnable_item(&mut self, pos: Position, index: u32, amount: u16, timer_set: u64) {
        self.spawnable_item.push(SpawnItemData {
            index,
            amount,
            pos,
            timer_set,
            ..Default::default()
        });
    }

    pub fn add_mapitem(&mut self, world: &mut World, mapitem: MapItem) -> Entity {
        let id = world.spawn((WorldEntityType::MapItem, mapitem));
        let _ = world.insert_one(id, EntityType::MapItem(Entity(id)));
        self.itemids.insert(Entity(id));
        Entity(id)
    }

    pub fn is_blocked_tile(&self, pos: Position, entity_type: WorldEntityType) -> bool {
        match self.move_grid[pos.as_tile()].attr {
            GridAttribute::Walkable => false,
            GridAttribute::Entity => self.move_grid[pos.as_tile()].count >= 5,
            GridAttribute::Blocked => true,
            GridAttribute::NpcBlock => entity_type == WorldEntityType::Npc,
        }
    }

    pub fn remove_entity_from_grid(&mut self, pos: Position) {
        self.move_grid[pos.as_tile()].count = self.move_grid[pos.as_tile()].count.saturating_sub(1);

        if self.move_grid[pos.as_tile()].count == 0 {
            self.move_grid[pos.as_tile()].attr = GridAttribute::Walkable;
        }
    }

    pub fn add_entity_to_grid(&mut self, pos: Position) {
        self.move_grid[pos.as_tile()].count = self.move_grid[pos.as_tile()].count.saturating_add(1);
        self.move_grid[pos.as_tile()].attr = GridAttribute::Entity;
    }

    pub fn add_player(&mut self, storage: &Storage, id: Entity) {
        self.players.insert(id);

        for i in self.get_surrounding(true) {
            if i != self.position {
                match storage.maps.get(&i) {
                    Some(map) => {
                        let count = map.borrow().players_on_map.saturating_add(1);
                        map.borrow_mut().players_on_map = count;
                    }
                    None => continue,
                }
            }
        }

        self.players_on_map = self.players_on_map.saturating_add(1);
    }

    pub fn add_npc(&mut self, id: Entity) {
        self.npcs.insert(id);
    }

    pub fn remove_player(&mut self, storage: &Storage, id: Entity) {
        self.players.swap_remove(&id);

        //we set the surrounding maps to have players on them if the player is within 1 map of them.
        for i in self.get_surrounding(true) {
            if i != self.position {
                match storage.maps.get(&i) {
                    Some(map) => {
                        let count = map.borrow().players_on_map.saturating_sub(1);
                        map.borrow_mut().players_on_map = count;
                    }
                    None => continue,
                }
            }
        }

        self.players_on_map = self.players_on_map.saturating_sub(1);
    }

    pub fn remove_npc(&mut self, id: Entity) {
        self.npcs.swap_remove(&id);
    }

    pub fn remove_item(&mut self, id: Entity) {
        /*if !self.items.contains(id) {
            return;
        }*/

        //self.items.remove(id);
        self.itemids.swap_remove(&id);
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
    storage: &Storage,
    position: MapPosition,
    dir: MapPosDir,
) -> Option<MapPosition> {
    let offset = position.map_offset(dir);
    let _ = storage.bases.maps.get(&offset)?;
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

    let map_positions = get_surrounding_dir(start.map, false);
    processed.insert(start.map);
    // lets check each surrounding map first to make sure its not here
    // before we span into the other maps.
    for map_pos in &map_positions {
        if map_pos.contains(endpos.map) {
            return Some(endpos.map_offset(map_pos.into()));
        }
    }

    //Else if not found above lets start searching within each side part ignoring
    //Maps not within the Allowed HashSet.
    for map_pos in &map_positions {
        let x = match map_pos.get() {
            Some(map_pos) => map_pos,
            None => continue,
        };

        if allowed_maps.get(&x).is_none() || processed.get(&x).is_some() {
            continue;
        }

        let end = endpos.map_offset(map_pos.into());
        let pos = Position::new(0, 0, x);
        let ret = map_offset_range(pos, end, allowed_maps, processed);

        //if it is Some then we did find it and get the offset so lets return it.
        if ret.is_some() {
            return ret;
        }
    }

    None
}

pub fn get_maps_in_range(storage: &Storage, pos: &Position, range: i32) -> Vec<MapPos> {
    let mut arr: Vec<MapPos> = Vec::new();

    if storage.bases.maps.get(&pos.map).is_none() {
        return Vec::new();
    }

    arr.push(MapPos::Center(pos.map));

    if pos.x - range < 0 && pos.y - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::UpLeft);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::UpLeft(pos));
        }
    }

    if pos.x - range < 0 && pos.y + range >= MAP_MAX_Y as i32 {
        let pos = pos.map.map_offset(MapPosDir::DownLeft);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::DownLeft(pos));
        }
    }

    if pos.x + range < 0 && pos.y - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::UpRight);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::UpRight(pos));
        }
    }

    if pos.x + range < 0 && pos.y + range >= MAP_MAX_Y as i32 {
        let pos = pos.map.map_offset(MapPosDir::DownRight);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::DownRight(pos));
        }
    }

    if pos.x - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::Left);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Left(pos));
        }
    }

    if pos.x + range >= MAP_MAX_X as i32 {
        let pos = pos.map.map_offset(MapPosDir::Right);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Right(pos));
        }
    }

    if pos.y - range < 0 {
        let pos = pos.map.map_offset(MapPosDir::Up);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Up(pos));
        }
    }

    if pos.y + range >= MAP_MAX_Y as i32 {
        let pos = pos.map.map_offset(MapPosDir::Down);

        if storage.bases.maps.get(&pos).is_some() {
            arr.push(MapPos::Down(pos));
        }
    }

    arr
}

pub fn map_path_blocked(
    storage: &Storage,
    cur_pos: Position,
    next_pos: Position,
    movedir: u8,
    entity_type: WorldEntityType,
) -> bool {
    // Directional blocking might be in the wrong order as it should be.
    // 0 down, 1 right, 2 up, 3 left
    //TODO: Sherwin check this please when you get a chance.
    let blocked = match movedir {
        0 => {
            if let Some(map) = storage.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].dir_block.get(B0) == 0b00000001
            } else {
                true
            }
        }
        1 => {
            if let Some(map) = storage.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].dir_block.get(B3) == 0b00001000
            } else {
                true
            }
        }
        2 => {
            if let Some(map) = storage.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].dir_block.get(B1) == 0b00000010
            } else {
                true
            }
        }
        _ => {
            if let Some(map) = storage.maps.get(&cur_pos.map) {
                map.borrow().move_grid[cur_pos.as_tile()].dir_block.get(B2) == 0b00000100
            } else {
                true
            }
        }
    };

    if !blocked {
        return match storage.maps.get(&next_pos.map) {
            Some(map) => map.borrow().is_blocked_tile(next_pos, entity_type),
            None => true,
        };
    }

    blocked
}
