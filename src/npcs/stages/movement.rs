use crate::{gametypes::*, npcs::*, time_ext::MyInstant, ClaimsKey};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum MovementStage {
    PathStart {
        npc_info: NpcInfo,
    },
    // first stage
    GetTargetUpdates {
        npc_info: NpcInfo,
        target: Targeting,
    },
    ClearTarget {
        npc_info: NpcInfo,
    },
    UpdateTarget {
        npc_info: NpcInfo,
        new_target: Targeting,
    },
    UpdateAStarPaths {
        npc_info: NpcInfo,
        timer: MyInstant,
        target_pos: Position,
    },
    UpdateRandPaths {
        npc_info: NpcInfo,
        timer: MyInstant,
    },
    ClearMovePath {
        npc_info: NpcInfo,
    },
    SetMovePath {
        npc_info: NpcInfo,
        timer: MyInstant,
        path: VecDeque<(Position, u8)>,
    },
    GetMoves {
        npc_info: NpcInfo,
        new_target: Targeting,
    },
    NextMove {
        npc_info: NpcInfo,
    },
    CheckBlock {
        npc_info: NpcInfo,
        next_move: (Position, u8),
    },
    ProcessMovement {
        npc_info: NpcInfo,
        next_move: (Position, u8),
    },
    ProcessTarget {
        npc_info: NpcInfo,
        target: Targeting,
        next_move: (Position, u8),
    },
    SetNpcDir {
        npc_info: NpcInfo,
        next_move: (Position, u8),
    },
    FinishMove {
        npc_info: NpcInfo,
        next_move: (Position, u8),
    },
    GetTileClaim {
        npc_info: NpcInfo,
        new_position: Position,
    },
    SwitchMaps {
        npc_info: NpcInfo,
        new_position: Position,
        can_switch: bool,
        map_switch_key: ClaimsKey,
    },
    MapSwitchFinish {
        npc_info: NpcInfo,
        new_position: Position,
        map_switch_key: ClaimsKey,
        npc: Box<Npc>,
    },
    MoveToCombat {
        npc_info: NpcInfo,
    },
}

impl MovementStage {
    pub fn send_map(&self) -> Option<MapPosition> {
        match self {
            MovementStage::MoveToCombat { npc_info }
            | MovementStage::SwitchMaps {
                npc_info,
                new_position: _,
                can_switch: _,
                map_switch_key: _,
            }
            | MovementStage::FinishMove {
                npc_info,
                next_move: _,
            }
            | MovementStage::SetNpcDir {
                npc_info,
                next_move: _,
            }
            | MovementStage::ProcessMovement {
                npc_info,
                next_move: _,
            }
            | MovementStage::CheckBlock {
                npc_info,
                next_move: _,
            }
            | MovementStage::NextMove { npc_info }
            | MovementStage::GetMoves {
                npc_info,
                new_target: _,
            }
            | MovementStage::SetMovePath {
                npc_info,
                timer: _,
                path: _,
            }
            | MovementStage::ClearMovePath { npc_info }
            | MovementStage::UpdateRandPaths { npc_info, timer: _ }
            | MovementStage::UpdateAStarPaths {
                npc_info,
                timer: _,
                target_pos: _,
            }
            | MovementStage::UpdateTarget {
                npc_info,
                new_target: _,
            }
            | MovementStage::ClearTarget { npc_info }
            | MovementStage::PathStart { npc_info } => Some(npc_info.position.map),
            MovementStage::ProcessTarget {
                npc_info: _,
                target,
                next_move: _,
            }
            | MovementStage::GetTargetUpdates {
                npc_info: _,
                target,
            } => {
                if let Some(pos) = target.get_pos() {
                    Some(pos.map)
                } else {
                    None
                }
            }
            MovementStage::MapSwitchFinish {
                npc_info: _,
                new_position,
                map_switch_key: _,
                npc: _,
            }
            | MovementStage::GetTileClaim {
                npc_info: _,
                new_position,
            } => Some(new_position.map),
        }
    }

    pub fn path_start(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Movement(MovementStage::PathStart { npc_info })
    }

    pub fn get_target_updates(npc_info: NpcInfo, target: Targeting) -> NpcStage {
        NpcStage::Movement(MovementStage::GetTargetUpdates { npc_info, target })
    }

    pub fn clear_target(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Movement(MovementStage::ClearTarget { npc_info })
    }

    pub fn update_target(npc_info: NpcInfo, new_target: Targeting) -> NpcStage {
        NpcStage::Movement(MovementStage::UpdateTarget {
            npc_info,
            new_target,
        })
    }

    pub fn update_astart_paths(
        npc_info: NpcInfo,
        timer: MyInstant,
        target_pos: Position,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::UpdateAStarPaths {
            npc_info,
            timer,
            target_pos,
        })
    }

    pub fn update_rand_paths(npc_info: NpcInfo, timer: MyInstant) -> NpcStage {
        NpcStage::Movement(MovementStage::UpdateRandPaths { npc_info, timer })
    }

    pub fn clear_move_path(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Movement(MovementStage::ClearMovePath { npc_info })
    }

    pub fn set_move_path(
        npc_info: NpcInfo,
        timer: MyInstant,
        path: VecDeque<(Position, u8)>,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::SetMovePath {
            npc_info,
            timer,
            path,
        })
    }

    pub fn get_moves(npc_info: NpcInfo, new_target: Targeting) -> NpcStage {
        NpcStage::Movement(MovementStage::GetMoves {
            npc_info,
            new_target,
        })
    }

    pub fn next_move(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Movement(MovementStage::NextMove { npc_info })
    }

    pub fn check_block(npc_info: NpcInfo, next_move: (Position, u8)) -> NpcStage {
        NpcStage::Movement(MovementStage::CheckBlock {
            npc_info,
            next_move,
        })
    }

    pub fn process_movement(npc_info: NpcInfo, next_move: (Position, u8)) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessMovement {
            npc_info,
            next_move,
        })
    }

    pub fn process_target(
        npc_info: NpcInfo,
        target: Targeting,
        next_move: (Position, u8),
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessTarget {
            npc_info,
            target,
            next_move,
        })
    }

    pub fn set_npc_dir(npc_info: NpcInfo, next_move: (Position, u8)) -> NpcStage {
        NpcStage::Movement(MovementStage::SetNpcDir {
            npc_info,
            next_move,
        })
    }

    pub fn finish_move(npc_info: NpcInfo, next_move: (Position, u8)) -> NpcStage {
        NpcStage::Movement(MovementStage::FinishMove {
            npc_info,
            next_move,
        })
    }

    pub fn get_tile_claim(npc_info: NpcInfo, new_position: Position) -> NpcStage {
        NpcStage::Movement(MovementStage::GetTileClaim {
            npc_info,
            new_position,
        })
    }

    pub fn switch_maps(
        npc_info: NpcInfo,
        new_position: Position,
        can_switch: bool,
        map_switch_key: ClaimsKey,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::SwitchMaps {
            npc_info,
            new_position,
            can_switch,
            map_switch_key,
        })
    }

    pub fn map_switch_finish(
        npc_info: NpcInfo,
        new_position: Position,
        map_switch_key: ClaimsKey,
        npc: Npc,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::MapSwitchFinish {
            npc_info,
            new_position,
            map_switch_key,
            npc: Box::new(npc),
        })
    }

    pub fn move_to_combat(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Movement(MovementStage::MoveToCombat { npc_info })
    }
}
