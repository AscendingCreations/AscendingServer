use crate::{
    gametypes::*,
    maps::{MapActor, MapActorStore},
    npcs::*,
};

#[derive(Debug, Clone)]
pub enum CombatStage {
    //First part of combat.
    BehaviourCheck {
        npc_info: NpcInfo,
    },
    CheckTarget {
        npc_info: NpcInfo,
        npc_mode: NpcMode,
        target: Target,
        cast_type: NpcCastType,
    },
    GetDefence {
        npc_info: NpcInfo,
        target: Target,
    },
    GetDamage {
        npc_info: NpcInfo,
        defense: u32,
        target: Target,
    },
    DoDamage {
        npc_info: NpcInfo,
        damage: i32,
        target: Target,
    },
    RemoveTarget {
        npc_info: NpcInfo,
    },
}

impl CombatStage {
    pub fn get_map(&self) -> Option<MapPosition> {
        match self {
            CombatStage::RemoveTarget { npc_info }
            | CombatStage::GetDamage {
                npc_info,
                defense: _,
                target: _,
            }
            | CombatStage::BehaviourCheck { npc_info } => Some(npc_info.position.map),
            CombatStage::DoDamage {
                npc_info: _,
                damage: _,
                target,
            }
            | CombatStage::GetDefence {
                npc_info: _,
                target,
            }
            | CombatStage::CheckTarget {
                npc_info: _,
                npc_mode: _,
                target,
                cast_type: _,
            } => target.get_pos().map(|pos| pos.map),
        }
    }

    pub fn behaviour_check(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Combat(CombatStage::BehaviourCheck { npc_info })
    }

    pub fn remove_target(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Combat(CombatStage::RemoveTarget { npc_info })
    }

    pub fn do_damage(npc_info: NpcInfo, damage: i32, target: Target) -> NpcStage {
        NpcStage::Combat(CombatStage::DoDamage {
            npc_info,
            damage,
            target,
        })
    }

    pub fn get_damage(npc_info: NpcInfo, defense: u32, target: Target) -> NpcStage {
        NpcStage::Combat(CombatStage::GetDamage {
            npc_info,
            defense,
            target,
        })
    }

    pub fn get_defense(npc_info: NpcInfo, target: Target) -> NpcStage {
        NpcStage::Combat(CombatStage::GetDefence { npc_info, target })
    }

    pub fn check_target(
        npc_info: NpcInfo,
        npc_mode: NpcMode,
        target: Target,
        cast_type: NpcCastType,
    ) -> NpcStage {
        NpcStage::Combat(CombatStage::CheckTarget {
            npc_info,
            npc_mode,
            target,
            cast_type,
        })
    }
}

pub async fn npc_combat(
    map: &mut MapActor,
    store: &mut MapActorStore,
    stage: CombatStage,
) -> Result<NpcStage> {
    let stage = match stage {
        CombatStage::BehaviourCheck { npc_info } => {
            if !npc_info.is_dead(map, store) {
                combat::behaviour_check(map, store, npc_info)?
            } else {
                NpcStage::None(npc_info)
            }
        }
        CombatStage::CheckTarget {
            npc_info,
            npc_mode,
            target,
            cast_type,
        } => {
            if !npc_info.is_dead(map, store) {
                combat::check_target(map, store, npc_info, npc_mode, target, cast_type)?
            } else {
                NpcStage::None(npc_info)
            }
        }
        CombatStage::GetDefence { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                combat::get_defense(store, npc_info, target)?
            } else {
                NpcStage::None(npc_info)
            }
        }
        CombatStage::GetDamage {
            npc_info,
            defense,
            target,
        } => {
            if !npc_info.is_dead(map, store) {
                combat::get_damage(store, npc_info, defense, target)?
            } else {
                NpcStage::None(npc_info)
            }
        }
        CombatStage::DoDamage {
            npc_info,
            damage,
            target,
        } => {
            if !npc_info.is_dead(map, store) {
                combat::do_damage(map, store, npc_info, damage, target).await?
            } else {
                NpcStage::None(npc_info)
            }
        }
        CombatStage::RemoveTarget { npc_info } => {
            if !npc_info.is_dead(map, store) {
                combat::remove_target(store, npc_info)?
            } else {
                NpcStage::None(npc_info)
            }
        }
    };

    Ok(stage)
}
