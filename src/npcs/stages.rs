mod combat;
mod movement;
mod targeting;

pub use combat::*;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
pub use movement::*;
use serde::{Deserialize, Serialize};
pub use targeting::TargetingStage;

use crate::maps::MapActor;

use super::{send_stage, NpcInfo};

#[derive(Debug, Clone, PartialEq)]
pub enum NpcStage {
    None(NpcInfo),
    Continue,
    Targeting(TargetingStage),
    Combat(CombatStage),
    Movement(MovementStage),
}

impl NpcStage {
    pub fn get_stages(&self) -> NpcStages {
        match self {
            NpcStage::None(_) | NpcStage::Continue => NpcStages::None,
            NpcStage::Targeting(_) => NpcStages::Targeting,
            NpcStage::Combat(_) => NpcStages::Combat,
            NpcStage::Movement(_) => NpcStages::Movement,
        }
    }
}

impl NpcStage {
    pub async fn send(self, map: &mut MapActor) {
        let map_pos = match &self {
            NpcStage::None(_) | NpcStage::Continue => return,
            NpcStage::Targeting(targeting) => targeting.send_map(),
            NpcStage::Combat(combat) => combat.send_map(),
            NpcStage::Movement(movement) => movement.send_map(),
        };

        if let Some(map_pos) = map_pos {
            if map_pos == map.position {
                map.npc_state_machine.push_back(self);
            } else {
                send_stage(map, map_pos, self).await
            }
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
