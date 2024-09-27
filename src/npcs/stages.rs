use crate::{gametypes::*, npcs::*, time_ext::MyInstant, ClaimsKey, GlobalKey};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;

pub enum NpcStage {
    None,
    Targeting(TargetingStage),
    Combat(CombatStage),
    Movement(MovementStage),
}

impl NpcStage {
    pub fn get_stages(&self) -> NpcStages {
        match self {
            NpcStage::None => NpcStages::None,
            NpcStage::Targeting(_) => NpcStages::Targeting,
            NpcStage::Combat(_) => NpcStages::Combat,
            NpcStage::Movement(_) => NpcStages::Movement,
        }
    }
}

pub enum CombatStage {
    None,
}

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
        npc: Arc<Mutex<Npc>>,
    },
    MoveToCombat {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
}

pub enum TargetingStage {
    None,
    // first stage
    CheckTarget {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
    },
    NpcDeTargetChance {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
    },
    CheckDistance {
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
    GetTargetMaps {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
    CheckMaps {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
    GetTargetFromMaps {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        // Map to pop Map position from to send to When we cant find any Targets.
        // If Vec is not null We pop till we run out of maps or we get a target.
        // If No Target Move NPC to Movement Stage.
        maps: Vec<MapPosition>,
    },
    SetTarget {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Target,
        target_pos: Position,
    },
    MoveToMovement {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
}
