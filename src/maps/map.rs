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
) -> MapDirPos {
    if start.group != position.group {
        return MapDirPos::None;
    }
    if position == start {
        MapDirPos::Center(position)
    } else if position == start.map_offset(MapDir::Up) {
        MapDirPos::Up(start.map_offset(MapDir::Up))
    } else if position == start.map_offset(MapDir::Down) {
        MapDirPos::Down(start.map_offset(MapDir::Down))
    } else if position == start.map_offset(MapDir::Left) {
        MapDirPos::Left(start.map_offset(MapDir::Left))
    } else if position == start.map_offset(MapDir::Right) {
        MapDirPos::Right(start.map_offset(MapDir::Right))
    } else if include_corners {
        if position == start.map_offset(MapDir::UpLeft) {
            MapDirPos::UpLeft(start.map_offset(MapDir::UpLeft))
        } else if position == start.map_offset(MapDir::UpRight) {
            MapDirPos::UpRight(start.map_offset(MapDir::UpRight))
        } else if position == start.map_offset(MapDir::DownLeft) {
            MapDirPos::DownLeft(start.map_offset(MapDir::DownLeft))
        } else if position == start.map_offset(MapDir::DownRight) {
            MapDirPos::DownRight(start.map_offset(MapDir::DownRight))
        } else {
            MapDirPos::None
        }
    } else {
        MapDirPos::None
    }
}

pub fn get_surrounding(position: MapPosition, include_corners: bool) -> Vec<MapPosition> {
    let mut arr = vec![
        position,
        position.map_offset(MapDir::Up),
        position.map_offset(MapDir::Down),
        position.map_offset(MapDir::Left),
        position.map_offset(MapDir::Right),
    ];

    if include_corners {
        arr.push(position.map_offset(MapDir::UpLeft));
        arr.push(position.map_offset(MapDir::UpRight));
        arr.push(position.map_offset(MapDir::DownLeft));
        arr.push(position.map_offset(MapDir::DownRight));
    }

    arr
}

pub fn get_surrounding_dir(position: MapPosition, include_corners: bool) -> Vec<MapDirPos> {
    let mut arr = vec![
        MapDirPos::Center(position),
        MapDirPos::Up(position.map_offset(MapDir::Up)),
        MapDirPos::Down(position.map_offset(MapDir::Down)),
        MapDirPos::Left(position.map_offset(MapDir::Left)),
        MapDirPos::Right(position.map_offset(MapDir::Right)),
    ];

    if include_corners {
        arr.push(MapDirPos::UpLeft(position.map_offset(MapDir::UpLeft)));
        arr.push(MapDirPos::UpRight(position.map_offset(MapDir::UpRight)));
        arr.push(MapDirPos::DownLeft(position.map_offset(MapDir::DownLeft)));
        arr.push(MapDirPos::DownRight(position.map_offset(MapDir::DownRight)));
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
    match dir {
        Dir::Down => {
            vec![
                position.map_offset(MapDir::Down),
                position.map_offset(MapDir::DownLeft),
                position.map_offset(MapDir::DownRight),
            ]
        }
        Dir::Right => {
            vec![
                position.map_offset(MapDir::Right),
                position.map_offset(MapDir::UpRight),
                position.map_offset(MapDir::DownRight),
            ]
        }
        Dir::Up => {
            vec![
                position.map_offset(MapDir::Up),
                position.map_offset(MapDir::UpLeft),
                position.map_offset(MapDir::UpRight),
            ]
        }
        Dir::Left => {
            vec![
                position.map_offset(MapDir::Left),
                position.map_offset(MapDir::UpLeft),
                position.map_offset(MapDir::DownLeft),
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

pub fn get_map_dir_pos_in_range(
    storage: &Storage,
    position: &Position,
    range: i32,
) -> Vec<MapDirPos> {
    let mut arr = Vec::with_capacity(10);
    let map_positions = get_surrounding_dir(position.map, true);

    for pos in map_positions {
        let map_dir = MapDir::from(&pos);

        let Some(map_position) = pos.get() else {
            continue;
        };

        if storage.bases.maps.get(&map_position).is_some()
            && match map_dir {
                MapDir::None => continue,
                MapDir::UpLeft => position.x - range < 0 && position.y - range < 0,
                MapDir::Up => position.y - range < 0,
                MapDir::UpRight => position.x + range < 0 && position.y - range < 0,
                MapDir::Left => position.x - range < 0,
                MapDir::Center => true,
                MapDir::Right => position.x + range >= MAP_MAX_X as i32,
                MapDir::DownLeft => {
                    position.x - range < 0 && position.y + range >= MAP_MAX_Y as i32
                }
                MapDir::Down => position.y + range >= MAP_MAX_Y as i32,
                MapDir::DownRight => {
                    position.x + range < 0 && position.y + range >= MAP_MAX_Y as i32
                }
            }
        {
            arr.push(pos);
        }
    }

    arr
}

pub fn get_map_pos_in_range(
    storage: &Storage,
    position: &Position,
    range: i32,
) -> Vec<MapPosition> {
    let mut arr = Vec::with_capacity(10);
    let map_positions = get_surrounding_dir(position.map, true);

    for pos in map_positions {
        let map_dir = MapDir::from(&pos);

        let Some(map_position) = pos.get() else {
            continue;
        };

        if storage.bases.maps.get(&map_position).is_some()
            && match map_dir {
                MapDir::None => continue,
                MapDir::UpLeft => position.x - range < 0 && position.y - range < 0,
                MapDir::Up => position.y - range < 0,
                MapDir::UpRight => position.x + range < 0 && position.y - range < 0,
                MapDir::Left => position.x - range < 0,
                MapDir::Center => true,
                MapDir::Right => position.x + range >= MAP_MAX_X as i32,
                MapDir::DownLeft => {
                    position.x - range < 0 && position.y + range >= MAP_MAX_Y as i32
                }
                MapDir::Down => position.y + range >= MAP_MAX_Y as i32,
                MapDir::DownRight => {
                    position.x + range < 0 && position.y + range >= MAP_MAX_Y as i32
                }
            }
        {
            arr.push(map_position);
        }
    }

    arr
}
