use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};

use crate::{maps::MapActor, MapPosition};

use super::PlayerInfo;

mod movement;

pub use movement::*;

#[derive(Debug, Clone)]
pub enum PlayerStage {
    None(PlayerInfo),
    Continue,
    Targeting(bool),
    Combat(bool),
    Movement(PlayerMovementStage),
}

impl PlayerStage {
    pub fn get_stages(&self) -> PlayerStages {
        match self {
            PlayerStage::None(_) | PlayerStage::Continue => PlayerStages::None,
            PlayerStage::Targeting(_) => PlayerStages::Targeting,
            PlayerStage::Combat(_) => PlayerStages::Combat,
            PlayerStage::Movement(_) => PlayerStages::Movement,
        }
    }
}

impl PlayerStage {
    pub async fn send(self, map: &mut MapActor) {
        let map_pos = match &self {
            PlayerStage::None(_) | PlayerStage::Continue => return,
            PlayerStage::Targeting(targeting) => todo!(),
            PlayerStage::Combat(combat) => todo!(),
            PlayerStage::Movement(movement) => movement.get_map(),
        };

        if let Some(map_pos) = map_pos {
            if map_pos == map.position {
                map.player_state_machine.push_back(self);
            } else {
                self.send_to_map(map, map_pos).await
            }
        }
    }

    pub async fn send_to_map(self, map: &MapActor, map_pos: MapPosition) {
        let sender = map.storage.map_senders.get(&map_pos).expect("Missing map?");

        sender
            .send(crate::maps::MapIncomming::PlayerStage {
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
pub enum PlayerStages {
    #[default]
    None,
    Targeting,
    Combat,
    Movement,
}
