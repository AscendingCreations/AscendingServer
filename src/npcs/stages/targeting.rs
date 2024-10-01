use crate::{gametypes::*, maps::MapActor, npcs::*};

#[derive(Debug, Clone, PartialEq)]
pub enum TargetingStage {
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
    pub fn get_target(&self) -> Target {
        match self {
            TargetingStage::CheckTarget {
                npc_info: _,
                target,
            } => target.target,
            TargetingStage::NpcDeTargetChance {
                npc_info: _,
                target,
            } => target.target,
            TargetingStage::CheckDistance {
                npc_info: _,
                target,
            } => target.target,
            _ => Target::None,
        }
    }

    pub fn send_map(&self) -> Option<MapPosition> {
        match self {
            TargetingStage::CheckDistance {
                npc_info: _,
                target,
            }
            | TargetingStage::NpcDeTargetChance {
                npc_info: _,
                target,
            } => {
                if let Some(pos) = target.get_pos() {
                    Some(pos.map)
                } else {
                    None
                }
            }
            TargetingStage::ClearTarget { npc_info }
            | TargetingStage::GetTargetMaps { npc_info }
            | TargetingStage::SetTarget {
                npc_info,
                target: _,
                target_pos: _,
            }
            | TargetingStage::MoveToMovement { npc_info } => Some(npc_info.position.map),
            _ => None,
        }
    }

    /*match data {
        Some((_, Some(_), true)) | Some((_, None, false)) | None => {}
        Some((info, Some(target), false)) => {
            if let Some(pos) = target.get_pos() {
                if pos.map == map.position {
                    map.npc_state_machine.push_back(self);
                }
            }
        }
        Some((info, None, true)) => {}
    }*/

    pub fn get_target_maps(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Targeting(TargetingStage::GetTargetMaps { npc_info })
    }

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
