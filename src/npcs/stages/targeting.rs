use crate::{gametypes::*, npcs::*, GlobalKey};
use std::sync::Arc;

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

impl TargetingStage {
    pub fn get_target_from_maps(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        maps: Vec<MapPosition>,
    ) -> NpcStage {
        NpcStage::Targeting(TargetingStage::GetTargetFromMaps {
            key,
            position,
            npc_data,
            maps,
        })
    }

    pub fn move_to_movement(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
    ) -> NpcStage {
        NpcStage::Targeting(TargetingStage::MoveToMovement {
            key,
            position,
            npc_data,
        })
    }

    pub fn set_target(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Target,
        target_pos: Position,
    ) -> NpcStage {
        NpcStage::Targeting(TargetingStage::SetTarget {
            key,
            position,
            npc_data,
            target,
            target_pos,
        })
    }

    pub fn check_target(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
    ) -> NpcStage {
        NpcStage::Targeting(TargetingStage::CheckTarget {
            key,
            position,
            npc_data,
            target,
        })
    }

    pub fn detarget_chance(
        key: GlobalKey,
        position: Position,
        npc_data: Arc<NpcData>,
        target: Targeting,
    ) -> NpcStage {
        NpcStage::Targeting(TargetingStage::NpcDeTargetChance {
            key,
            position,
            npc_data,
            target,
        })
    }

    pub fn clear_target(key: GlobalKey, position: Position, npc_data: Arc<NpcData>) -> NpcStage {
        NpcStage::Targeting(TargetingStage::ClearTarget {
            key,
            position,
            npc_data,
        })
    }
}
