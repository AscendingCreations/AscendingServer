use crate::{
    gametypes::*,
    maps::{MapActor, MapActorStore},
    npcs::*,
};

#[derive(Debug, Clone)]
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

    pub fn get_map(&self) -> Option<MapPosition> {
        match self {
            TargetingStage::CheckDistance {
                npc_info: _,
                target,
            }
            | TargetingStage::NpcDeTargetChance {
                npc_info: _,
                target,
            } => target.get_pos().map(|pos| pos.map),
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

pub async fn npc_targeting(
    map: &mut MapActor,
    store: &mut MapActorStore,
    stage: TargetingStage,
) -> Result<NpcStage> {
    let stage = match stage {
        TargetingStage::CheckTarget { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                targeting::check_target(store, npc_info, target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::NpcDeTargetChance { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                targeting::check_detargeting(map, npc_info, target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::CheckDistance { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                targeting::check_target_distance(store, npc_info, target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::ClearTarget { npc_info } => {
            if !npc_info.is_dead(map, store) {
                targeting::clear_target(store, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::GetTargetMaps { npc_info } => {
            if !npc_info.is_dead(map, store) {
                targeting::get_targeting_maps(map, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::GetTargetFromMaps { npc_info, mut maps } => {
            if !npc_info.is_dead(map, store) {
                let stage = targeting::get_target(store, npc_info);

                if let NpcStage::Targeting(TargetingStage::MoveToMovement { npc_info }) = stage {
                    if let Some(next_map) = maps.pop() {
                        TargetingStage::get_target_from_maps(npc_info, maps)
                            .send_to_map(map, next_map)
                            .await;
                        NpcStage::Continue
                    } else {
                        TargetingStage::move_to_movement(npc_info)
                    }
                } else {
                    stage
                }
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::SetTarget {
            npc_info,
            target,
            target_pos,
        } => {
            if !npc_info.is_dead(map, store) {
                targeting::set_target(map, store, npc_info, target, target_pos)
            } else {
                NpcStage::None(npc_info)
            }
        }
        TargetingStage::MoveToMovement { npc_info } => {
            if !npc_info.is_dead(map, store)
                && let Some(npc) = store.npcs.get_mut(&npc_info.key)
            {
                if npc_info.data.can_move {
                    npc.stage = NpcStages::Movement;
                    MovementStage::path_start(npc_info)
                } else if npc_info.data.can_attack {
                    npc.stage = NpcStages::Combat;
                    CombatStage::behaviour_check(npc_info)
                } else {
                    NpcStage::None(npc_info)
                }
            } else {
                NpcStage::None(npc_info)
            }
        }
    };

    Ok(stage)
}
