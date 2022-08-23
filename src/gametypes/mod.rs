mod entity;
mod enums;
mod error;
mod map_position;
mod position;
mod rgb;
mod sharedstructs;

pub use entity::*;
pub use enums::*;
pub use error::{AraisealError, Result};
pub use map_position::*;
pub use position::*;
pub use rgb::Rgba;
pub use sharedstructs::*;

pub const SKILL_MAX: usize = SkillStat::Count as usize;
pub const COMBAT_MAX: usize = CombatStat::Count as usize;
pub const EQUIPMENT_TYPE_MAX: usize = EquipmentType::Count as usize;
pub const VITALS_MAX: usize = VitalTypes::Count as usize;

pub type Combatstats = [i16; COMBAT_MAX];
pub type BuffCombatstats = [i16; COMBAT_MAX];
pub type CombatstatExps = [u64; COMBAT_MAX];
pub type Skillstats = [i16; SKILL_MAX];
pub type BuffSkillstats = [i16; SKILL_MAX];
pub type SkillstatExps = [u64; SKILL_MAX];

pub const SERVERCONNECTION: &str = "0.0.0.0:7010";
pub const MAXCONNECTIONS: usize = 500;
pub const DATABASE: &str =
    "user=MainServer hostaddr=127.0.0.1 port=5432 password=testy dbname=test";
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
pub const MAX_QUESTS: usize = 500;
pub const MAX_RESOURCES: usize = 1000;
pub const MAX_CRAFTS: usize = 1000;
pub const MAX_PLAYERS: usize = 1000;

pub const MAX_STAT_LVL: usize = 101;
pub const MAX_LVL: usize = 200;
pub const MAX_INV: usize = 378;
pub const MAX_SKILL_INV: usize = 11;
pub const MAX_ITEM_VAL: usize = 999;
pub const MAX_BANK_ITEMS: usize = 90;
pub const MAX_BANK_SLOTS: usize = 4;
pub const MAX_FRIENDS: usize = 50;
pub const MAX_NAME_LENGTH: usize = 32;
pub const START_MAP: u32 = 21;
pub const START_X: u32 = 17;
pub const START_Y: u32 = 15;
pub const MAX_TITLES: usize = 256;
pub const MAX_SKILL_SLOTS: usize = 60;
pub const MAX_PARTY_SIZE: usize = 12;

pub const DIR_UP: usize = 0;
pub const DIR_RIGHT: usize = 1;
pub const DIR_DOWN: usize = 2;
pub const DIR_LEFT: usize = 3;
