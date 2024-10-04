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

    pub fn get_map_position(&self) -> Option<MapPosition> {
        match &self {
            NpcStage::Continue => None,
            NpcStage::Targeting(targeting) => targeting.get_map(),
            NpcStage::Combat(combat) => combat.get_map(),
            NpcStage::Movement(movement) => movement.get_map(),
            NpcStage::None(info) => Some(info.position.map),
        }
    }

    pub fn get_npc_info(self) -> Option<NpcInfo> {
        match self {
            NpcStage::Continue => None,
            NpcStage::Targeting(targeting) => Some(targeting.get_npc_info()),
            NpcStage::Combat(combat) => Some(combat.get_npc_info()),
            NpcStage::Movement(movement) => Some(movement.get_npc_info()),
            NpcStage::None(info) => Some(info),
        }
    }

    pub fn get_owner_map(&self) -> Option<MapPosition> {
        match &self {
            NpcStage::Continue => None,
            NpcStage::Targeting(targeting) => Some(targeting.get_owner_map()),
            NpcStage::Combat(combat) => Some(combat.get_owner_map()),
            NpcStage::Movement(movement) => Some(movement.get_owner_map()),
            NpcStage::None(info) => Some(info.position.map),
        }
    }
}

impl NpcStage {
    pub async fn send(self, map: &mut MapActor) {
        let map_pos = self.get_map_position();

        if let Some(map_pos) = map_pos {
            if map_pos == map.position {
                map.npc_state_machine.push_back(self);
            } else if map.storage.map_senders.contains_key(&map_pos) {
                self.send_to_map(map, map_pos).await
            } else if let Some(map_pos) = self.get_owner_map() {
                let stages = self.get_stages();
                let npc_info = self.get_npc_info().unwrap();

                let stage = match stages {
                    NpcStages::None => {
                        log::error!("NPC Info pointed to non existant map, did it get unloaded?");
                        return;
                    }
                    NpcStages::Targeting => TargetingStage::clear_target(npc_info),
                    NpcStages::Combat => CombatStage::remove_target(npc_info),
                    NpcStages::Movement => MovementStage::clear_move_path(npc_info),
                };

                if map_pos == map.position {
                    map.npc_state_machine.push_back(stage);
                } else if map.storage.map_senders.contains_key(&map_pos) {
                    stage.send_to_map(map, map_pos).await
                } else {
                    log::error!("Could not send packet to owner map did it get unloaded?");
                }
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
