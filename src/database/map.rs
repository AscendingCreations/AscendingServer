use std::fs::{self, OpenOptions};
use std::io::BufReader;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::database::map;

const MAP_PATH: &str = "./data/maps/";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum MapAttribute {
    Walkable,
    Blocked,
    Warp(i32, i32, u64, u32, u32),
    Sign(String),
    Count,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub id: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapData {
    pub x: i32,
    pub y: i32,
    pub group: u64,
    pub tile: Vec<Tile>,
    pub attribute: Vec<MapAttribute>,
    pub zonespawns: [Vec<(u16, u16)>; 5],
    pub zones: [(u64, [Option<u64>; 5]); 5],
    pub fixed_weather: u8,
}

impl MapData {
    pub fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            group: 0,
            tile: vec![Tile { id: vec![0; 1024] }; 9],
            attribute: vec![MapAttribute::Walkable; 1024],
            zonespawns: Default::default(),
            zones: Default::default(),
            fixed_weather: 0,
        }
    }
}

pub fn get_maps() -> Vec<MapData> {
    let entries = fs::read_dir(MAP_PATH).unwrap();

    let mut map_data: Vec<MapData> = Vec::new();

    for entry in entries {
        if let Ok(entry_data) = entry {
            if let Ok(filename) = entry_data.file_name().into_string() {
                if let Some(mapdata) = load_file(filename) {
                    map_data.push(mapdata);
                }
            }
        }
    }

    map_data
}

fn load_file(filename: String) -> Option<MapData> {
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