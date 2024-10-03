use crate::{containers::*, gametypes::*, GlobalKey};
use educe::Educe;
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use std::path::Path;
use std::{
    fs::{self, OpenOptions},
    io::Read,
};

const MAP_PATH: &str = "./data/maps/";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Readable, Writable)]
pub struct WarpData {
    pub map_x: i32,
    pub map_y: i32,
    pub map_group: u64,
    pub tile_x: u32,
    pub tile_y: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Readable, Writable)]
pub struct ItemSpawnData {
    pub index: u32,
    pub amount: u16,
    pub timer: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Readable, Writable)]
pub enum MapAttribute {
    #[default]
    Walkable,
    Blocked,
    NpcBlocked,
    Warp(WarpData),
    Sign(String),
    ItemSpawn(ItemSpawnData),
    Storage,
    Shop(u16),
    Count,
}

/// The Block Type per Tile. This does not include Attributes so you will need
/// to cycle through the Static Map Attribute Vec to get that information.
/// this is only used to deturmine if something is blocked or not.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GridAttribute {
    #[default]
    Walkable,
    GlobalKey,
    Blocked,
    NpcBlock,
}

/// Data that is changable per Tile for Blocking purposes within MapData
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GridTile {
    pub count: u8,
    pub attr: GridAttribute,
    pub item: Option<(GlobalKey, u32, u16)>,
    pub dir_block: u8,
}

//TODO: Update to use MAP (x,y,group) for map locations and Remove map links?
#[derive(Clone, Educe, Serialize, Deserialize, Readable, Writable, Debug)]
#[educe(Default(new))]
pub struct Map {
    pub position: MapPosition,
    pub dir_block: Vec<u8>,
    pub attribute: Vec<MapAttribute>,
    // Tiles for zone spawning. (x, y) using u8 to cut down the size and since maps should never Exceed 64x64
    // As super large maps are stupid within a Seamless Structure.
    pub zonespawns: [Vec<(u16, u16)>; 5],
    // (Max spawns per zone, [npc_id; 5])
    pub zones: [(u64, [Option<u64>; 5]); 5],
    pub music: Option<String>,
    pub weather: Weather,
}

impl Map {
    pub fn get_surrounding(&self, include_corners: bool) -> Vec<MapPosition> {
        get_surrounding(self.position, include_corners)
    }
}

pub fn load_maps() -> Vec<Map> {
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
        Ok(mut file) => {
            let mut bytes = Vec::new();
            match file.read_to_end(&mut bytes) {
                Ok(_) => Some(Map::read_from_buffer(&bytes).unwrap()),
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

pub fn check_surrounding(
    start: MapPosition,
    position: MapPosition,
    include_corners: bool,
) -> MapPos {
    if start.group != position.group {
        return MapPos::None;
    }
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

pub fn get_surrounding_set(position: MapPosition) -> HashSet<MapPosition> {
    let mut set = HashSet::<MapPosition>::default();

    for next_position in get_surrounding(position, true) {
        set.insert(next_position);
    }

    set
}

/// Gets maps based on the players direction from their old map position.
/// Should Speed up map sending to only maps that need it.
pub fn get_directional_maps(position: MapPosition, dir: Dir) -> Vec<MapPosition> {
    //Down: 0, Right: 1, Up: 2, Left: 3

    match dir {
        Dir::Down => {
            vec![
                position.map_offset(MapPosDir::Down),
                position.map_offset(MapPosDir::DownLeft),
                position.map_offset(MapPosDir::DownRight),
            ]
        }
        Dir::Right => {
            vec![
                position.map_offset(MapPosDir::Right),
                position.map_offset(MapPosDir::UpRight),
                position.map_offset(MapPosDir::DownRight),
            ]
        }
        Dir::Up => {
            vec![
                position.map_offset(MapPosDir::Up),
                position.map_offset(MapPosDir::UpLeft),
                position.map_offset(MapPosDir::UpRight),
            ]
        }
        Dir::Left => {
            vec![
                position.map_offset(MapPosDir::Left),
                position.map_offset(MapPosDir::UpLeft),
                position.map_offset(MapPosDir::DownLeft),
            ]
        }
    }
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

    if start.map.group != endpos.map.group {
        return None;
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
