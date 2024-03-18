use crate::containers::Storage;
use crate::gametypes::*;
//use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::BufReader;

#[derive(Derivative, Clone, Debug, Default, Serialize, Deserialize)]
#[derivative(PartialEq)]
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
    pub drops: [(u32, u32, u32); 10], //item dropped on death, chance, amount
    pub drops_max: u16, //number of Different items that will be picked  0..=drops_max. that we can cycle thru and drop at random.
}

impl NpcData {
    /*pub fn new_npc(&self, spawn: Position, zone: usize, npc_id: u64) -> Npc {
        Npc {
            num: npc_id,
            spawntimer: MyInstant::from_dur(self.spawn_wait),
            spawned_zone: Some(zone),
            e: self.new_entity(spawn),
            ..Default::default()
        }
    }

    pub fn new_entity(&self, spawn: Position) -> Entity {
        let mut rng = thread_rng();

        Entity {
            spawn,
            level: self.level,
            pos: spawn,
            vital: [self.maxhp as i32, self.maxmp as i32, self.maxsp as i32],
            vitalmax: [self.maxhp as i32, self.maxmp as i32, self.maxsp as i32],
            pdamage: self.pdamage,
            pdefense: self.pdefense,
            life: DeathType::UnSpawned,
            dir: rng.gen_range(0..4),
            mode: NpcMode::Normal,
            ..Default::default()
        }
    }*/

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
    
    /// load npc data from json with serdes.
    /// if ID does not exist or file nto found return None.
    pub fn load_npc(storage: &Storage, id: u64) -> Option<NpcData> {
        let npc_data = storage.bases.npcs.get(id as usize);
        if let Some(data) = npc_data {
            return Some(data.clone());
        }
        None
    }
}

pub fn get_npc() -> Vec<NpcData> {
    let mut npc_data: Vec<NpcData> = Vec::new();

    let mut count = 0;
    let mut got_data = true;

    while got_data {
        if let Some(data) = load_file(count) {
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
    let name = format!("./data/npcs/{}.json", id);

    match OpenOptions::new().read(true).open(&name) {
        Ok(file) => {
            let reader = BufReader::new(file);

            match serde_json::from_reader(reader) {
                Ok(v) => Some(v),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}
