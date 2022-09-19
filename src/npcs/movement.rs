use crate::{containers::Storage, gametypes::*, maps::*, npcs::*};
use chrono::Duration;

pub fn movement(world: &Storage, npc: &mut Npc, _base: &NpcData) {
    //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
    if npc.ai_timer < *world.gettick.borrow() && npc.moving {
        npc.moves.clear();
        npc.moving = false;
    }

    if !npc.moving && npc.moves.is_empty() {
        if let Some(movepos) = npc.move_pos {
            //Move pos overrides targeting pos movement.
            if let Some(path) = a_star_path(world, npc.e.pos, npc.e.dir, movepos) {
                npc.set_move_path(path);
            }
        } else if npc.e.targettype != EntityType::None
            && world
                .maps
                .get(&npc.e.pos.map)
                .map(|map| map.borrow().players_on_map())
                .unwrap_or(false)
        {
            if let Some(path) = a_star_path(world, npc.e.pos, npc.e.dir, npc.e.targetpos) {
                npc.set_move_path(path);
            }
        } else {
            //no special movement lets give them some if we can;
            if world
                .maps
                .get(&npc.e.pos.map)
                .map(|map| map.borrow().players_on_map())
                .unwrap_or(false)
            {
                npc.set_move_path(npc_rand_movement(world, npc.e.pos, npc.e.dir));
            }
        }

        npc.ai_timer = *world.gettick.borrow() + Duration::milliseconds(2500);
    }

    if npc.moving {
        let next = match npc.moves.pop() {
            Some(v) => v,
            None => return,
        };

        if map_path_blocked(world, npc.e.pos, next.0, next.1) {
            npc.moves.push(next);
            return;
        }

        if npc.e.pos == next.0 {
            npc.set_npc_dir(world, next.1);
        } else {
            if npc.move_pos.is_none() {
                //do any movepos to position first
                if !world
                    .maps
                    .get(&npc.e.pos.map)
                    .map(|map| map.borrow().players_on_map())
                    .unwrap_or(false)
                {
                    npc.clear_move_path();
                    return;
                }

                match npc.e.targettype {
                    EntityType::Player(i, accid) => {
                        if let Some(target) = world.players.borrow().get(i as usize) {
                            let target = target.borrow();

                            if target.e.life.is_alive() && target.accid == accid {
                                if target.e.pos == next.0 {
                                    npc.clear_move_path();
                                    npc.set_npc_dir(world, next.1);
                                    return;
                                }
                            } else {
                                npc.clear_move_path();
                            }
                        } else {
                            npc.clear_move_path();
                        }
                    }
                    EntityType::Npc(i) => {
                        if let Some(target) = world.npcs.borrow().get(i as usize) {
                            if target.borrow().e.life.is_alive() {
                                if target.borrow().e.pos == next.0 {
                                    npc.clear_move_path();
                                    npc.set_npc_dir(world, next.1);
                                    return;
                                }
                            } else {
                                npc.clear_move_path();
                            }
                        } else {
                            npc.clear_move_path();
                        }
                    }
                    _ => {}
                };
            } else if next.0 == npc.move_pos.unwrap() {
                npc.move_pos = None;
                npc.clear_move_path();
            }

            npc.e.dir = next.1;

            if next.0.map != npc.e.pos.map {
                npc.switch_maps(world, next.0);

                //TODO: send map switch here
                // Let client know what npc and to what map. let the client do the full removal
            } else {
                npc.swap_pos(world, next.0);
                //TODO: NPC move packet here
            }
        }
    }
}
