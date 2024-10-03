use crate::{
    gametypes::*,
    maps::{MapActor, MapActorStore},
    npcs::*,
    time_ext::MyInstant,
    ClaimsKey,
};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum MovementStage {
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
        path: VecDeque<(Position, Dir)>,
    },
    GetMoves {
        npc_info: NpcInfo,
        new_target: Targeting,
    },
    NextMove {
        npc_info: NpcInfo,
    },
    CheckBlock {
        npc_info: NpcInfo,
        next_move: (Position, Dir),
    },
    ProcessMovement {
        npc_info: NpcInfo,
        next_move: (Position, Dir),
    },
    ProcessTarget {
        npc_info: NpcInfo,
        target: Targeting,
        next_move: (Position, Dir),
    },
    SetNpcDir {
        npc_info: NpcInfo,
        next_move: (Position, Dir),
    },
    FinishMove {
        npc_info: NpcInfo,
        next_move: (Position, Dir),
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
    pub fn get_map(&self) -> Option<MapPosition> {
        match self {
            MovementStage::MoveToCombat { npc_info }
            | MovementStage::SwitchMaps {
                npc_info,
                new_position: _,
                can_switch: _,
                map_switch_key: _,
            }
            | MovementStage::FinishMove {
                npc_info,
                next_move: _,
            }
            | MovementStage::SetNpcDir {
                npc_info,
                next_move: _,
            }
            | MovementStage::ProcessMovement {
                npc_info,
                next_move: _,
            }
            | MovementStage::CheckBlock {
                npc_info,
                next_move: _,
            }
            | MovementStage::NextMove { npc_info }
            | MovementStage::GetMoves {
                npc_info,
                new_target: _,
            }
            | MovementStage::SetMovePath {
                npc_info,
                timer: _,
                path: _,
            }
            | MovementStage::ClearMovePath { npc_info }
            | MovementStage::UpdateRandPaths { npc_info, timer: _ }
            | MovementStage::UpdateAStarPaths {
                npc_info,
                timer: _,
                target_pos: _,
            }
            | MovementStage::UpdateTarget {
                npc_info,
                new_target: _,
            }
            | MovementStage::ClearTarget { npc_info }
            | MovementStage::PathStart { npc_info } => Some(npc_info.position.map),
            MovementStage::ProcessTarget {
                npc_info: _,
                target,
                next_move: _,
            }
            | MovementStage::GetTargetUpdates {
                npc_info: _,
                target,
            } => target.get_pos().map(|pos| pos.map),
            MovementStage::MapSwitchFinish {
                npc_info: _,
                new_position,
                map_switch_key: _,
                npc: _,
            }
            | MovementStage::GetTileClaim {
                npc_info: _,
                new_position,
            } => Some(new_position.map),
        }
    }

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
        path: VecDeque<(Position, Dir)>,
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::SetMovePath {
            npc_info,
            timer,
            path,
        })
    }

    pub fn get_moves(npc_info: NpcInfo, new_target: Targeting) -> NpcStage {
        NpcStage::Movement(MovementStage::GetMoves {
            npc_info,
            new_target,
        })
    }

    pub fn next_move(npc_info: NpcInfo) -> NpcStage {
        NpcStage::Movement(MovementStage::NextMove { npc_info })
    }

    pub fn check_block(npc_info: NpcInfo, next_move: (Position, Dir)) -> NpcStage {
        NpcStage::Movement(MovementStage::CheckBlock {
            npc_info,
            next_move,
        })
    }

    pub fn process_movement(npc_info: NpcInfo, next_move: (Position, Dir)) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessMovement {
            npc_info,
            next_move,
        })
    }

    pub fn process_target(
        npc_info: NpcInfo,
        target: Targeting,
        next_move: (Position, Dir),
    ) -> NpcStage {
        NpcStage::Movement(MovementStage::ProcessTarget {
            npc_info,
            target,
            next_move,
        })
    }

    pub fn set_npc_dir(npc_info: NpcInfo, next_move: (Position, Dir)) -> NpcStage {
        NpcStage::Movement(MovementStage::SetNpcDir {
            npc_info,
            next_move,
        })
    }

    pub fn finish_move(npc_info: NpcInfo, next_move: (Position, Dir)) -> NpcStage {
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

pub async fn npc_movement(
    map: &mut MapActor,
    store: &mut MapActorStore,
    stage: MovementStage,
) -> Result<NpcStage> {
    let stage = match stage {
        MovementStage::PathStart { npc_info } => {
            if !npc_info.is_dead(map, store) {
                movement::path_start(map, store, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::GetTargetUpdates { npc_info, target } => {
            if !npc_info.is_dead(map, store) {
                movement::get_target_updates(store, npc_info, target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::ClearTarget { npc_info } => {
            if !npc_info.is_dead(map, store) {
                movement::clear_target(store, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::UpdateTarget {
            npc_info,
            new_target,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::update_target(map, store, npc_info, new_target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::UpdateAStarPaths {
            npc_info,
            timer,
            target_pos,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::update_astar_paths(map, store, npc_info, timer, target_pos)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::UpdateRandPaths { npc_info, timer } => {
            if !npc_info.is_dead(map, store) {
                movement::update_rand_paths(map, store, npc_info, timer)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::ClearMovePath { npc_info } => {
            if !npc_info.is_dead(map, store) {
                movement::clear_move_path(store, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::SetMovePath {
            npc_info,
            timer,
            path,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::set_move_path(store, npc_info, timer, path)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::GetMoves {
            npc_info,
            new_target,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::get_moves(map, store, npc_info, new_target)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::NextMove { npc_info } => {
            if !npc_info.is_dead(map, store) {
                movement::next_move(store, npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::CheckBlock {
            npc_info,
            next_move,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::check_block(map, store, npc_info, next_move)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::ProcessMovement {
            npc_info,
            next_move,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::process_movement(map, store, npc_info, next_move)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::ProcessTarget {
            npc_info,
            target,
            next_move,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::process_target(store, npc_info, target, next_move)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::SetNpcDir {
            npc_info,
            next_move,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::set_npc_dir(map, store, npc_info, next_move)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::FinishMove {
            npc_info,
            next_move,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::finish_movement(map, store, npc_info, next_move)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::GetTileClaim {
            npc_info,
            new_position,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::get_tile_claim(store, npc_info, new_position)
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::SwitchMaps {
            npc_info,
            new_position,
            can_switch,
            map_switch_key,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::switch_maps(
                    map,
                    store,
                    npc_info,
                    new_position,
                    map_switch_key,
                    can_switch,
                )
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::MapSwitchFinish {
            npc_info,
            new_position,
            map_switch_key,
            npc,
        } => {
            if !npc_info.is_dead(map, store) {
                movement::finish_map_switch(
                    map,
                    store,
                    npc_info,
                    new_position,
                    map_switch_key,
                    *npc,
                )
            } else {
                NpcStage::None(npc_info)
            }
        }
        MovementStage::MoveToCombat { npc_info } => {
            if !npc_info.is_dead(map, store)
                && npc_info.data.can_attack
                && let Some(npc) = store.npcs.get_mut(&npc_info.key)
            {
                npc.stage = NpcStages::Combat;
                CombatStage::behaviour_check(npc_info)
            } else {
                NpcStage::None(npc_info)
            }
        }
    };

    Ok(stage)
}
