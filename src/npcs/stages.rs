use crate::{gametypes::*, npcs::*, GlobalKey};
use std::sync::Arc;

pub enum NpcStage {
    None,
    Targeting(TargetingStage),
    Combat,
    Movement,
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
    },
    MoveToMovement {
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    },
}
