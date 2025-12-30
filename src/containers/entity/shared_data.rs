use educe::Educe;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};

use crate::{
    containers::GlobalKey,
    gametypes::{MapPosition, Position, VITALS_MAX},
    time_ext::MyInstant,
};

#[derive(Debug, Clone, Default)]
pub struct MovementData {
    pub spawn: Spawn,
    pub pos: Position,
    pub dir: u8,
    pub move_timer: MoveTimer,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Spawn {
    #[educe(Default = Position::new(10, 10, MapPosition::new(0,0,0)))]
    pub pos: Position,
    #[educe(Default  = MyInstant::recent())]
    pub just_spawned: MyInstant,
}

#[derive(Debug, Clone, Default)]
pub struct CombatData {
    pub vitals: Vitals,
    pub level: i32,
    pub in_combat: bool,
    pub stunned: bool,
    pub attacking: bool,
    pub death_type: DeathType,
    pub target: Target,
    pub attack_timer: AttackTimer,
    pub death_timer: DeathTimer,
    pub combat_timer: CombatTimer,
    pub physical: Physical,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq, MByteBufferWrite, MByteBufferRead)]
#[educe(Default)]
pub struct Vitals {
    #[educe(Default = [25, 2, 100])]
    pub vital: [i32; VITALS_MAX],
    #[educe(Default = [25, 2, 100])]
    pub vitalmax: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub regens: [u32; VITALS_MAX],
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct AttackTimer(#[educe(Default = MyInstant::recent())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct DeathTimer(#[educe(Default = MyInstant::recent())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct CombatTimer(#[educe(Default = MyInstant::recent())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct MoveTimer(#[educe(Default = MyInstant::recent())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Target {
    pub target_entity: Option<GlobalKey>,
    pub target_pos: Position,
    #[educe(Default = MyInstant::recent())]
    pub target_timer: MyInstant,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Physical {
    pub damage: u32,
    pub defense: u32,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum DeathType {
    #[default]
    Alive,
    Spirit,
    Dead,
    Spawning,
}

impl DeathType {
    pub fn is_dead(self) -> bool {
        !matches!(self, DeathType::Alive)
    }

    pub fn is_spirit(self) -> bool {
        matches!(self, DeathType::Spirit)
    }

    pub fn is_alive(self) -> bool {
        matches!(self, DeathType::Alive)
    }

    pub fn is_spawning(self) -> bool {
        matches!(self, DeathType::Spawning)
    }
}
