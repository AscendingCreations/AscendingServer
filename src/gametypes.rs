mod enums;
mod error;
mod map_position;
mod position;
mod rgb;
mod sharedstructs;

pub use enums::*;
pub use error::{AscendingError, Result};
pub use map_position::*;
pub use position::*;
pub use rgb::Rgba;
pub use sharedstructs::*;

pub const EQUIPMENT_TYPE_MAX: usize = EquipmentType::Count as usize;
pub const VITALS_MAX: usize = VitalTypes::Count as usize;

pub const MAXCONNECTIONS: usize = 500;
pub const APP_MAJOR: usize = 1;
pub const APP_MINOR: usize = 1;
pub const APP_REVISION: usize = 1;

///Map Data Maxs
pub const MAX_MAPS: usize = 3000;
pub const MAP_MAX_X: usize = 32;
pub const MAP_MAX_Y: usize = 32;
pub const MAX_TILE: usize = MAP_MAX_X * MAP_MAX_Y - 1;

///Array Data Maxs
pub const MAX_NPCS: usize = 1000;
pub const MAX_ITEMS: usize = 2000;
pub const MAX_SHOPS: usize = 100;
pub const MAX_PLAYERS: usize = 1000;
pub const MAX_SOCKET_PLAYERS: usize = 2000;

pub const MAX_WORLD_NPCS: usize = 100_000;
pub const NPCS_SPAWNCAP: usize = 10;

pub const MAX_LVL: usize = 200;
pub const MAX_INV: usize = 30;
pub const MAX_TRADE_SLOT: usize = 30;
pub const MAX_STORAGE: usize = 70;
pub const MAX_EQPT: usize = 5;
pub const MAX_ITEM_VAL: usize = 999;
pub const MAX_NAME_LENGTH: usize = 32;
pub const START_MAP: u32 = 21;
pub const START_X: u32 = 17;
pub const START_Y: u32 = 15;
pub const MAX_PARTY_SIZE: usize = 12;
pub const MAX_SHOP_ITEM: usize = 20;

pub const DIR_UP: usize = 0;
pub const DIR_RIGHT: usize = 1;
pub const DIR_DOWN: usize = 2;
pub const DIR_LEFT: usize = 3;
