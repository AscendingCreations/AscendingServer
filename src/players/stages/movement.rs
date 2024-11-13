use super::PlayerStage;
use crate::{
    gametypes::*,
    players::{Player, PlayerInfo},
    ClaimsKey,
};

#[derive(Debug, Clone)]
pub enum PlayerMovementStage {
    None(PlayerInfo),
    GetNewPosition {
        info: PlayerInfo,
        dir: Dir,
    },
    CheckBlocked {
        info: PlayerInfo,
        next_move: (Position, Dir),
    },
    SendToOriginalLocation {
        info: PlayerInfo,
        dir: Dir,
    },
    StartPlayerWarp {
        info: PlayerInfo,
        next_move: (Position, Dir),
    },
    FinishPlayerWarp {
        info: PlayerInfo,
        next_move: (Position, Dir),
        player: Box<Player>,
    },
    StartMapSwitch {
        info: PlayerInfo,
        next_move: (Position, Dir),
        claim: ClaimsKey,
    },
    FinishMapSwitch {
        info: PlayerInfo,
        next_move: (Position, Dir),
        claim: ClaimsKey,
        player: Box<Player>,
    },
    MoveToPosition {
        info: PlayerInfo,
        next_move: (Position, Dir),
    },
}

impl PlayerMovementStage {
    pub fn none(info: PlayerInfo) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::None(info))
    }

    pub fn get_new_position(info: PlayerInfo, dir: Dir) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::GetNewPosition { info, dir })
    }

    pub fn start_player_warp(info: PlayerInfo, next_move: (Position, Dir)) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::StartPlayerWarp { info, next_move })
    }

    pub fn check_blocked(info: PlayerInfo, next_move: (Position, Dir)) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::CheckBlocked { info, next_move })
    }

    pub fn move_to_position(info: PlayerInfo, next_move: (Position, Dir)) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::CheckBlocked { info, next_move })
    }

    pub fn send_to_original_location(info: PlayerInfo, dir: Dir) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::SendToOriginalLocation { info, dir })
    }

    pub fn start_map_switch(
        info: PlayerInfo,
        next_move: (Position, Dir),
        claim: ClaimsKey,
    ) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::StartMapSwitch {
            info,
            next_move,
            claim,
        })
    }

    pub fn finish_map_switch(
        info: PlayerInfo,
        next_move: (Position, Dir),
        claim: ClaimsKey,
        player: Player,
    ) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::FinishMapSwitch {
            info,
            next_move,
            claim,
            player: Box::new(player),
        })
    }

    pub fn finish_player_warp(
        info: PlayerInfo,
        next_move: (Position, Dir),
        player: Player,
    ) -> PlayerStage {
        PlayerStage::Movement(PlayerMovementStage::FinishPlayerWarp {
            info,
            next_move,
            player: Box::new(player),
        })
    }

    pub fn get_map(&self) -> Option<MapPosition> {
        match self {
            PlayerMovementStage::None(player_info) => Some(player_info.position.map),
            PlayerMovementStage::GetNewPosition { info, dir: _ }
            | PlayerMovementStage::SendToOriginalLocation { info, dir: _ }
            | PlayerMovementStage::StartPlayerWarp { info, next_move: _ }
            | PlayerMovementStage::StartMapSwitch {
                info,
                next_move: _,
                claim: _,
            }
            | PlayerMovementStage::MoveToPosition { info, next_move: _ } => Some(info.position.map),
            PlayerMovementStage::FinishMapSwitch {
                info: _,
                next_move,
                claim: _,
                player: _,
            }
            | PlayerMovementStage::FinishPlayerWarp {
                info: _,
                next_move,
                player: _,
            }
            | PlayerMovementStage::CheckBlocked { info: _, next_move } => Some(next_move.0.map),
        }
    }
}
