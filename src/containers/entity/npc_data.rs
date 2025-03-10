use std::collections::VecDeque;

use educe::Educe;

use crate::{
    gametypes::{NpcMode, Position},
    time_ext::MyInstant,
};

use super::{CombatData, MovementData, Sprite};

#[derive(Debug, Clone, Default)]
pub struct NpcEntity {
    // General Data
    pub sprite: Sprite,
    pub index: u64,
    pub despawns: bool,
    pub moving: bool,
    pub retreating: bool,
    pub walk_to_spawn: bool,
    pub mode: NpcMode,

    // Location
    pub movement: MovementData,
    pub moves: NpcMoves,
    pub move_pos: NpcMovePos,
    pub spawned_zone: NpcSpawnedZone,

    // Combat
    pub combat: CombatData,
    pub hit_by: NpcHitBy,

    // Timer
    pub timer: NpcTimer,
    pub ai_timer: NpcAITimer,
    pub path_timer: NpcPathTimer,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcTimer {
    #[educe(Default = MyInstant::now())]
    pub despawntimer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub spawntimer: MyInstant,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcAITimer(#[educe(Default = MyInstant::now())] pub MyInstant); //for rebuilding the a* paths

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcPathTimer {
    #[educe(Default = MyInstant::now())]
    pub timer: MyInstant,
    pub tries: usize,
    //when failing to move due to blocks in movement.
    pub fails: usize,
} //for rebuilding the a* paths

#[derive(Educe, Debug, Clone, PartialEq, Eq)]
#[educe(Default)]
//offset for special things so the npc wont to events based on this spawn time.
pub struct NpcHitBy(#[educe(Default = Vec::new())] pub Vec<(u32, u64, u64)>);

#[derive(Educe, Debug, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcMoves(#[educe(Default = VecDeque::new())] pub VecDeque<(Position, u8)>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcSpawnedZone(pub Option<usize>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcMovePos(pub Option<Position>);
