use crate::{gametypes::*, npcs::*, time_ext::MyInstant, ClaimsKey};
use std::collections::VecDeque;

pub enum MovementStage {
    None,
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
    ProcessMovePosition {
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

    pub fn process_move_position(npc_info: NpcInfo, new_target: Targeting) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessMovePosition {
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
