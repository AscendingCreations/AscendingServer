use super::{CombatData, MovementData, Sprite};
use crate::gametypes::Position;
use educe::Educe;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use time::Instant;

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
    #[educe(Default = Instant::recent())]
    pub despawntimer: Instant,
    #[educe(Default = Instant::recent())]
    pub spawntimer: Instant,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcAITimer(#[educe(Default = Instant::recent())] pub Instant); //for rebuilding the a* paths

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcPathTimer {
    #[educe(Default = Instant::recent())]
    pub timer: Instant,
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

#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Default,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum NpcMode {
    None,
    #[default]
    Normal,
    Pet,
    Summon,
    Boss,
}
