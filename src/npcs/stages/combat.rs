use crate::{gametypes::*, npcs::*, GlobalKey};
use std::sync::Arc;

pub enum CombatStage {
    None,
    BehaviourCheck {
        npc_info: NpcInfo,
    },
    CheckTarget {
        npc_info: NpcInfo,
        npc_mode: NpcMode,
        target: Target,
        cast_type: NpcCastType,
    },
}

impl CombatStage {
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
