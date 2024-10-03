mod combat;
mod movement;
mod targeting;

pub use combat::*;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
pub use movement::*;
use serde::{Deserialize, Serialize};
pub use targeting::*;

use crate::{maps::MapActor, MapPosition};

use super::NpcInfo;

#[derive(Debug, Clone)]
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
            NpcStage::Targeting(targeting) => targeting.get_map(),
            NpcStage::Combat(combat) => combat.get_map(),
            NpcStage::Movement(movement) => movement.get_map(),
        };

        if let Some(map_pos) = map_pos {
            if map_pos == map.position {
                map.npc_state_machine.push_back(self);
            } else {
                self.send_to_map(map, map_pos).await
            }
        }
    }

    pub async fn send_to_map(self, map: &MapActor, map_pos: MapPosition) {
        let sender = map.storage.map_senders.get(&map_pos).expect("Missing map?");

        sender
            .send(crate::maps::MapIncomming::NpcStage {
                map_id: map.position,
                stage: self,
            })
            .await
            .expect("Could not send to map. means map got unloaded?");
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
