use crate::{gametypes::*, npcs::*};

pub enum TargetingStage {
    None,
    // first stage
    CheckTarget {
        npc_info: NpcInfo,
        target: Targeting,
    },
    NpcDeTargetChance {
        npc_info: NpcInfo,
        target: Targeting,
    },
    CheckDistance {
        npc_info: NpcInfo,
        target: Targeting,
    },
    ClearTarget {
        npc_info: NpcInfo,
    },
    GetTargetMaps {
        npc_info: NpcInfo,
    },
    GetTargetFromMaps {
        npc_info: NpcInfo,
        // Map to pop Map position from to send to When we cant find any Targets.
        // If Vec is not null We pop till we run out of maps or we get a target.
        // If No Target Move NPC to Movement Stage.
        maps: Vec<MapPosition>,
    },
    SetTarget {
        npc_info: NpcInfo,
        target: Target,
        target_pos: Position,
    },
    MoveToMovement {
        npc_info: NpcInfo,
    },
}

impl TargetingStage {
    pub fn get_target_from_maps(npc_info: NpcInfo, maps: Vec<MapPosition>) -> NpcStage {
        NpcStage::Targeting(TargetingStage::GetTargetFromMaps { npc_info, maps })
    }

    pub fn move_to_movement(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Targeting(TargetingStage::MoveToMovement { npc_info })
    }

    pub fn set_target(npc_info: NpcInfo, target: Target, target_pos: Position) -> NpcStage {
        NpcStage::Targeting(TargetingStage::SetTarget {
            npc_info,
            target,
            target_pos,
        })
    }

    pub fn check_target(npc_info: NpcInfo, target: Targeting) -> NpcStage {
        NpcStage::Targeting(TargetingStage::CheckTarget { npc_info, target })
    }

    pub fn detarget_chance(npc_info: NpcInfo, target: Targeting) -> NpcStage {
        NpcStage::Targeting(TargetingStage::NpcDeTargetChance { npc_info, target })
    }

    pub fn clear_target(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Targeting(TargetingStage::ClearTarget { npc_info })
    }
}
