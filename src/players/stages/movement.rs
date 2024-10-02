use crate::{
    gametypes::*,
    maps::{MapActor, MapActorStore},
    npcs::*,
    players::PlayerInfo,
    time_ext::MyInstant,
    ClaimsKey,
};
use std::collections::VecDeque;

use super::PlayerStage;

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerMovementStage {
    GetNewPosition {
        info: PlayerInfo,
        dir: u8,
    },
    CheckBlocked {
        info: PlayerInfo,
        next_move: (Position, u8),
    },
    SendToOriginalLocation {
        info: PlayerInfo,
        dir: u8,
    },
    StartPlayerWarp {
        info: PlayerInfo,
        next_move: (Position, u8),
    },
    StartMapSwitch {
        info: PlayerInfo,
        next_move: (Position, u8),
        claim: ClaimsKey,
    },

    MoveToPosition {
        info: PlayerInfo,
        next_move: (Position, u8),
    },
}

impl PlayerMovementStage {
    pub fn get_new_position(info: PlayerInfo, dir: u8) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::GetNewPosition { info, dir })
    }

    pub fn start_player_warp(info: PlayerInfo, next_move: (Position, u8)) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::StartPlayerWarp { info, next_move })
    }

    pub fn check_blocked(info: PlayerInfo, next_move: (Position, u8)) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::CheckBlocked { info, next_move })
    }

    pub fn move_to_position(info: PlayerInfo, next_move: (Position, u8)) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::CheckBlocked { info, next_move })
    }

    pub fn send_to_original_location(info: PlayerInfo, dir: u8) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::SendToOriginalLocation { info, dir })
    }

    pub fn start_map_switch(
        info: PlayerInfo,
        next_move: (Position, u8),
        claim: ClaimsKey,
    ) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::StartMapSwitch {
            info,
            next_move,
            claim,
        })
    }
}
