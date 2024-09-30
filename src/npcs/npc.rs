use crate::{
    gametypes::*,
    maps::MapActor,
    npcs::NpcStages,
    tasks::{dir_packet, DataTaskToken},
    time_ext::MyInstant,
    GlobalKey,
};
use chrono::Duration;
use educe::Educe;
use std::{collections::VecDeque, sync::Arc};

use super::NpcData;

#[derive(Debug, Clone, PartialEq, Eq, Educe)]
#[educe(Default)]
pub struct Npc {
    pub index: u64,
    pub key: GlobalKey,
    #[educe(Default = MyInstant::now())]
    pub spawn_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub ai_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub path_timer: MyInstant,
    pub path_tries: usize,
    //when failing to move due to blocks in movement.
    pub path_fails: usize,
    #[educe(Default = false)]
    pub despawn: bool,
    #[educe(Default = false)]
    pub moving: bool,
    #[educe(Default = false)]
    pub retreating: bool,
    #[educe(Default = VecDeque::new())]
    pub moves: VecDeque<(Position, u8)>,
    pub spawn_zone: Option<usize>,
    pub move_pos_overide: Option<Position>,
    #[educe(Default = Position::new(10, 10, MapPosition::new(0,0,0)))]
    pub spawn_pos: Position,
    #[educe(Default  = MyInstant::now())]
    pub just_spawned: MyInstant,
    pub target: Targeting,
    pub kill_count: u32,
    #[educe(Default = MyInstant::now())]
    pub kill_count_timer: MyInstant,
    #[educe(Default = [25, 2, 100])]
    pub vital: [i32; VITALS_MAX],
    #[educe(Default = [25, 2, 100])]
    pub vitalmax: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub regens: [u32; VITALS_MAX],
    pub dir: u8,
    #[educe(Default = MyInstant::now())]
    pub despawn_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub attack_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub death_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub move_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub combat_timer: MyInstant,
    pub damage: u32,
    pub defense: u32,
    pub data: [i64; 10],
    pub hidden: bool,
    pub stunned: bool,
    pub attacking: bool,
    pub in_combat: bool,
    #[educe(Default = 1)]
    pub level: i32,
    pub position: Position,
    pub access: UserAccess,
    pub death: Death,
    pub is_using: IsUsingType,
    pub mode: NpcMode,
    pub sprite: u16,
    pub stage: NpcStages,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct NpcMapInfo {
    pub index: u64,
    pub key: GlobalKey,
    pub position: Position,
    pub death: Death,
    pub dir: u8,
}

impl NpcMapInfo {
    pub fn new_from(npc: &Npc) -> Self {
        Self {
            index: npc.index,
            key: npc.key,
            position: npc.position,
            death: npc.death,
            dir: npc.dir,
        }
    }
}

#[inline(always)]
pub fn is_npc_same(from_entity: &GlobalKey, to_entity: &GlobalKey) -> bool {
    from_entity == to_entity
}

impl Npc {
    pub fn new_from(
        map: &MapActor,
        index: u64,
        spawn_pos: Position,
        spawn_zone: Option<usize>,
    ) -> Option<Self> {
        map.storage.get_npc(index).map(|npc_data| Self {
            index,
            key: GlobalKey::default(),
            spawn_timer: map.tick
                + Duration::try_milliseconds(npc_data.spawn_wait).unwrap_or_default(),
            spawn_zone,
            spawn_pos,
            position: spawn_pos,
            mode: NpcMode::Normal,
            death: Death::Spawning,
            ..Default::default()
        })
    }

    #[inline(always)]
    pub fn npc_set_move_path(&mut self, path: VecDeque<(Position, u8)>) {
        self.moves = path;
        self.moving = true;
    }

    #[inline(always)]
    pub fn npc_clear_move_path(&mut self) {
        self.moves.clear();
        self.path_fails = 0;
        self.path_tries = 0;
        self.moving = false;
    }

    #[inline(always)]
    pub fn reset_path_tries(&mut self, timer: MyInstant) {
        self.path_fails = 0;
        self.path_tries = 0;
        self.path_timer = timer;
    }

    #[inline(always)]
    pub fn set_npc_dir(&mut self, map: &mut MapActor, dir: u8) -> Result<()> {
        if self.dir != dir {
            self.dir = dir;

            DataTaskToken::Dir.add_task(map, dir_packet(self.key, dir)?)?;
        }

        Ok(())
    }

    #[inline(always)]
    pub fn npc_swap_pos(&mut self, map: &mut MapActor, pos: Position) -> Position {
        let oldpos = self.position;
        if oldpos != pos {
            self.position = pos;

            map.remove_entity_from_grid(oldpos);
            map.add_entity_to_grid(pos);
        }

        oldpos
    }

    pub async fn npc_getx(&self) -> i32 {
        self.position.x
    }

    pub async fn npc_gety(&self) -> i32 {
        self.position.y
    }

    pub async fn npc_getmap(&self) -> MapPosition {
        self.position.map
    }

    pub async fn npc_gethp(&self) -> i32 {
        self.vital[VitalTypes::Hp as usize]
    }

    pub async fn npc_setx(&mut self, x: i32) {
        self.position.x = x;
    }

    pub async fn npc_sety(&mut self, y: i32) {
        self.position.y = y;
    }

    pub async fn npc_setmap(&mut self, map: MapPosition) {
        self.position.map = map;
    }

    pub async fn npc_sethp(&mut self, hp: i32) {
        self.vital[VitalTypes::Hp as usize] = hp;
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NpcInfo {
    pub key: GlobalKey,
    pub position: Position,
    pub data: Arc<NpcData>,
}

impl NpcInfo {
    pub fn new(key: GlobalKey, position: Position, npc_data: Arc<NpcData>) -> Self {
        Self {
            key,
            position,
            data: npc_data,
        }
    }
}
