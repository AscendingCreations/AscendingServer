use crate::{gametypes::*, npcs::*, time_ext::MyInstant, ClaimsKey, GlobalKey};
use std::{collections::VecDeque, sync::Arc};

pub enum MovementStage {
    None,
    PathStart {
        key: GlobalKey,
        npc_data: Arc<NpcData>,
    },
    // first stage
    GetTargetUpdates {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
    },
    ClearTarget {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
    UpdateTarget {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        new_target: Targeting,
    },
    UpdateAStarPaths {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        timer: MyInstant,
        target_pos: Position,
    },
    UpdateRandPaths {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        timer: MyInstant,
    },
    ClearMovePath {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
    SetMovePath {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        timer: MyInstant,
        path: VecDeque<(Position, u8)>,
    },
    ProcessMovePosition {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        new_target: Targeting,
    },
    NextMove {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
    CheckBlock {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    },
    ProcessMovement {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    },
    ProcessTarget {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
        next_move: (Position, u8),
    },
    SetNpcDir {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    },
    FinishMove {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    },
    GetTileClaim {
        key: GlobalKey,
        old_position: Position,
        npc_data: Arc<NpcData>,
        new_position: Position,
    },
    SwitchMaps {
        key: GlobalKey,
        old_position: Position,
        npc_data: Arc<NpcData>,
        new_position: Position,
        can_switch: bool,
        map_switch_key: ClaimsKey,
    },
    MapSwitchFinish {
        key: GlobalKey,
        npc_data: Arc<NpcData>,
        new_position: Position,
        map_switch_key: ClaimsKey,
        npc: Npc,
    },
    MoveToCombat {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
}

impl MovementStage {
    pub fn path_start(key: GlobalKey, npc_data: Arc<NpcData>) -> NpcStage {
        NpcStage::Movement(MovementStage::PathStart { key, npc_data })
    }

    pub fn get_target_updates(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::GetTargetUpdates {
            key,
            position,
            npc_data,
            target,
        })
    }

    pub fn clear_target(key: GlobalKey, position: Position, npc_data: Arc<NpcData>) -> NpcStage {
        NpcStage::Movement(MovementStage::ClearTarget {
            key,
            position,
            npc_data,
        })
    }

    pub fn update_target(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        new_target: Targeting,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::UpdateTarget {
            key,
            position,
            npc_data,
            new_target,
        })
    }

    pub fn update_astart_paths(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        timer: MyInstant,
        target_pos: Position,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::UpdateAStarPaths {
            key,
            position,
            npc_data,
            timer,
            target_pos,
        })
    }

    pub fn update_rand_paths(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        timer: MyInstant,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::UpdateRandPaths {
            key,
            position,
            npc_data,
            timer,
        })
    }

    pub fn clear_move_path(key: GlobalKey, position: Position, npc_data: Arc<NpcData>) -> NpcStage {
        NpcStage::Movement(MovementStage::ClearMovePath {
            key,
            position,
            npc_data,
        })
    }

    pub fn set_move_path(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        timer: MyInstant,
        path: VecDeque<(Position, u8)>,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::SetMovePath {
            key,
            position,
            npc_data,
            timer,
            path,
        })
    }

    pub fn process_move_position(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        new_target: Targeting,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessMovePosition {
            key,
            position,
            npc_data,
            new_target,
        })
    }

    pub fn next_move(key: GlobalKey, position: Position, npc_data: Arc<NpcData>) -> NpcStage {
        NpcStage::Movement(MovementStage::NextMove {
            key,
            position,
            npc_data,
        })
    }

    pub fn check_block(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::CheckBlock {
            key,
            position,
            npc_data,
            next_move,
        })
    }

    pub fn process_movement(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessMovement {
            key,
            position,
            npc_data,
            next_move,
        })
    }

    pub fn process_target(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
        next_move: (Position, u8),
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessTarget {
            key,
            position,
            npc_data,
            target,
            next_move,
        })
    }

    pub fn set_npc_dir(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::SetNpcDir {
            key,
            position,
            npc_data,
            next_move,
        })
    }

    pub fn finish_move(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        next_move: (Position, u8),
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::FinishMove {
            key,
            position,
            npc_data,
            next_move,
        })
    }

    pub fn get_tile_claim(
        key: GlobalKey,
        old_position: Position,
        npc_data: Arc<NpcData>,
        new_position: Position,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::GetTileClaim {
            key,
            old_position,
            npc_data,
            new_position,
        })
    }

    pub fn switch_maps(
        key: GlobalKey,
        old_position: Position,
        npc_data: Arc<NpcData>,
        new_position: Position,
        can_switch: bool,
        map_switch_key: ClaimsKey,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::SwitchMaps {
            key,
            old_position,
            npc_data,
            new_position,
            can_switch,
            map_switch_key,
        })
    }

    pub fn map_switch_finish(
        key: GlobalKey,
        npc_data: Arc<NpcData>,
        new_position: Position,
        map_switch_key: ClaimsKey,
        npc: Npc,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::MapSwitchFinish {
            key,
            npc_data,
            new_position,
            map_switch_key,
            npc,
        })
    }

    pub fn move_to_combat(key: GlobalKey, position: Position, npc_data: Arc<NpcData>) -> NpcStage {
        NpcStage::Movement(MovementStage::MoveToCombat {
            key,
            position,
            npc_data,
        })
    }
}
