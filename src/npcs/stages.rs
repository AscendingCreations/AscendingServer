mod combat;
mod movement;
mod targeting;

pub use combat::*;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
pub use movement::*;
use serde::{Deserialize, Serialize};
pub use targeting::TargetingStage;

use super::NpcInfo;

pub enum NpcStage {
    None(NpcInfo),
    Targeting(TargetingStage),
    Combat(CombatStage),
    Movement(MovementStage),
}

impl NpcStage {
    pub fn get_stages(&self) -> NpcStages {
        match self {
            NpcStage::None(_) => NpcStages::None,
            NpcStage::Targeting(_) => NpcStages::Targeting,
            NpcStage::Combat(_) => NpcStages::Combat,
            NpcStage::Movement(_) => NpcStages::Movement,
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Deserialize,
    Serialize,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum NpcStages {
    #[default]
    None,
    Targeting,
    Combat,
    Movement,
}
