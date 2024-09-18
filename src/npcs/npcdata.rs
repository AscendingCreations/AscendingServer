use crate::containers::Storage;
use crate::gametypes::*;
use educe::Educe;
use rangemap::RangeMap;
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use std::fs::OpenOptions;
use std::io::Read;

#[derive(Educe, Clone, Debug, Default, Serialize, Deserialize, Readable, Writable)]
#[educe(PartialEq)]
pub struct DropItemData {
    pub item: u32,
    pub amount: u32,
}

#[derive(Educe, Clone, Debug, Default, Serialize, Deserialize, Readable, Writable)]
#[educe(PartialEq)]
pub struct NpcDrop {
    pub items: [DropItemData; 5],
    pub shares: u32,
}

#[derive(Educe, Clone, Debug, Default, Serialize, Deserialize, Readable, Writable)]
#[educe(PartialEq)]
pub struct NpcData {
    pub name: String,
    pub level: i32,
    pub sprite: i32,
    pub respawn_wait: i64,
    pub movement_wait: i64,
    pub attack_wait: i64,
    pub intervaled_wait: i64,
    pub spawn_wait: i64,
    pub maxhp: u32,
    pub maxsp: u32,
    pub maxmp: u32,
    pub sight: i32,
    pub follow_sight: i32,
    pub walkdistance: u32,
    pub pdamage: u32,
    pub pdefense: u32,
    pub canpassthru: bool,
    pub size: TileBox,
    pub behaviour: AIBehavior,
    pub maxdamage: u32,
    pub mindamage: u32,
    pub target_auto_switch: bool,
    pub target_attacked_switch: bool,
    pub target_auto_switch_chance: i64,
    pub target_range_dropout: bool,
    pub can_target: bool,
    pub can_move: bool,
    pub can_attack_player: bool,
    pub has_allys: bool,
    pub has_enemies: bool,
    pub can_attack: bool,
    pub has_selfonly: bool,
    pub has_friendonly: bool,
    pub has_groundonly: bool,
    pub runsaway: bool,
    pub isanimated: bool,
    pub run_damage: u32,
    pub spawntime: (GameTime, GameTime),
    pub range: i32,        //attack range. How far they need to be to hit their target.
    pub enemies: Vec<u64>, //list of enemies the npcs can attack of other npc's... WAR!
    pub drops: [NpcDrop; 10],
    pub free_shares: u32,
    pub exp: i64,
    // Drop Data
    #[speedy(skip)]
    pub drop_ranges: RangeMap<u32, usize>,
    #[speedy(skip)]
    pub max_shares: u32,
}

impl NpcData {
    pub fn is_agressive(&self) -> bool {
        self.behaviour.is_agressive()
    }

    pub fn is_reactive(&self) -> bool {
        self.behaviour.is_reactive()
    }

    pub fn is_healer(&self) -> bool {
        self.behaviour.is_healer()
    }

    pub fn is_friendly(&self) -> bool {
        self.behaviour.is_friendly()
    }
}

pub fn load_npcs() -> Vec<NpcData> {
    let mut npc_data: Vec<NpcData> = Vec::new();

    let mut count = 0;
    let mut got_data = true;

    while got_data {
        if let Some(mut data) = load_file(count) {
            // Setup drop
            let mut pos = 0;
            let mut max_shares = 0;
            for (slot_id, drops) in data.drops.iter().enumerate() {
                if drops.shares == 0 {
                    continue;
                }

                max_shares += drops.shares;

                let range = pos..pos + drops.shares;
                pos += drops.shares;
                data.drop_ranges.insert(range, slot_id);
            }
            max_shares += data.free_shares;
            data.max_shares = max_shares;

            npc_data.push(data);
            count += 1;
            got_data = true;
        } else {
            got_data = false;
        }
    }

    npc_data
}

fn load_file(id: u64) -> Option<NpcData> {
    let name = format!("./data/npcs/{}.bin", id);

    match OpenOptions::new().read(true).open(name) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            match file.read_to_end(&mut bytes) {
                Ok(_) => Some(NpcData::read_from_buffer(&bytes).unwrap()),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}
