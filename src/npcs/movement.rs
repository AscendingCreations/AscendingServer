use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, players::Account, tasks::*};
use chrono::Duration;

pub fn npc_movement(world: &mut hecs::World, storage: &Storage, entity: &Entity, _base: &NpcData) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    //AI Timer is used to Reset the Moves every so offten to recalculate them for possible changes.
    if data.get::<&NpcAITimer>().expect("Could not find NpcAITimer").0 < *storage.gettick.borrow() &&
        data.get::<&NpcMoving>().expect("Could not find NpcMoving").0 {
        if let mut moves = data.get::<&mut NpcMoves>().expect("Could not find NpcMoves")
            { moves.0.clear() }
        if let mut moving = data.get::<&mut NpcMoving>().expect("Could not find NpcMoves")
            { moving.0 = false }
    }

    if !data.get::<&NpcMoving>().expect("Could not find NpcMoving").0 &&
        data.get::<&NpcMoves>().expect("Could not find NpcMoves").0.is_empty() {
        if let Some(movepos) = data.get::<&NpcMovePos>().expect("Could not find NpcMovePos").0 {
            //Move pos overrides targeting pos movement.
            if let Some(path) = 
                a_star_path(storage, 
                    *data.get::<&Position>().expect("Could not find Position"),
                    data.get::<&Dir>().expect("Could not find Dir").0, movepos) {
                npc_set_move_path(world, entity, path);
            }
        } else if data.get::<&Target>().expect("Could not find Target").targettype != EntityType::None
            && storage
                .maps
                .get(&data.get::<&Position>().expect("Could not find Position").map)
                .map(|map| map.borrow().players_on_map())
                .unwrap_or(false)
        {
            if let Some(path) = 
                a_star_path(storage, 
                    *data.get::<&Position>().expect("Could not find Position"),
                    data.get::<&Dir>().expect("Could not find Dir").0,
                    data.get::<&Target>().expect("Could not find Target").targetpos) {
                npc_set_move_path(world, entity, path);
            }
        } else {
            //no special movement lets give them some if we can;
            if storage
                .maps
                .get(&data.get::<&Position>().expect("Could not find Position").map)
                .map(|map| map.borrow().players_on_map())
                .unwrap_or(false)
            {
                npc_set_move_path(world, entity, 
                    npc_rand_movement(storage, 
                            *data.get::<&Position>().expect("Could not find Position"),
                            data.get::<&Dir>().expect("Could not find Dir").0));
            }
        }

        if let mut npcaitimer = data.get::<&mut NpcAITimer>().expect("Could not find NpcAITimer")
            { npcaitimer.0 = *storage.gettick.borrow() + Duration::milliseconds(2500) }
    }

    if data.get::<&NpcMoving>().expect("Could not find NpcMoving").0 {
        let next = match data.get::<&mut NpcMoves>().expect("Could not find NpcMoves").0.pop() {
            Some(v) => v,
            None => return,
        };

        if map_path_blocked(storage, 
            *data.get::<&Position>().expect("Could not find Position"), 
            next.0, next.1) {
            if let mut npcmoves = data.get::<&mut NpcMoves>().expect("Could not find NpcMoves")
                { npcmoves.0.push(next) }
            return;
        }

        if *data.get::<&Position>().expect("Could not find Position") == next.0 {
            set_npc_dir(world, storage, entity, next.1);
        } else {
            if data.get::<&NpcMovePos>().expect("Could not find NpcMovePos").0.is_none() {
                //do any movepos to position first
                if !storage
                    .maps
                    .get(&data.get::<&Position>().expect("Could not find Position").map)
                    .map(|map| map.borrow().players_on_map())
                    .unwrap_or(false)
                {
                    npc_clear_move_path(world, entity);
                    return;
                }

                match data.get::<&Target>().expect("Could not find Target").targettype {
                    EntityType::Player(i, accid) => {
                        if let Ok(tdata) = world.entity(i.0) {
                            if tdata.get::<&DeathType>().expect("Could not find DeathType").is_alive() && 
                                tdata.get::<&Account>().expect("Could not find Account").id == accid {
                                if *tdata.get::<&Position>().expect("Could not find Position") == next.0 {
                                    npc_clear_move_path(world, entity);
                                    set_npc_dir(world, storage, entity, next.1);
                                    return;
                                }
                            } else {
                                npc_clear_move_path(world, entity);
                            }
                        } else {
                            npc_clear_move_path(world, entity);
                        }
                    }
                    EntityType::Npc(i) => {
                        if let Ok(tdata) = world.entity(i.0) {
                            if tdata.get::<&DeathType>().expect("Could not find DeathType").is_alive() {
                                if *tdata.get::<&Position>().expect("Could not find Position") == next.0 {
                                    npc_clear_move_path(world, entity);
                                    set_npc_dir(world, storage, entity, next.1);
                                    return;
                                }
                            } else {
                                npc_clear_move_path(world, entity);
                            }
                        } else {
                            npc_clear_move_path(world, entity);
                        }
                    }
                    _ => {}
                };
            } else if next.0 == data.get::<&NpcMovePos>().expect("Could not find NpcMovePos").0.unwrap() {
                if let mut movepos = data.get::<&mut NpcMovePos>().expect("Could not find NpcMovePos")
                    { movepos.0 = None }
                npc_clear_move_path(world, entity);
            }

            if let mut dir = data.get::<&mut Dir>().expect("Could not find Dir")
                { dir.0 = next.1 }

            if next.0.map != data.get::<&Position>().expect("Could not find Position").map {
                let old_map = data.get::<&Position>().expect("Could not find Position").map;
                npc_switch_maps(world, storage, entity, next.0);
                //Send this Twice one to the old map and one to the new. Just in case people in outermaps did not get it yet.
                let _ = DataTaskToken::NpcMove(old_map).add_task(
                    world,
                    storage,
                    &MovePacket::new(*entity, next.0, false, true, next.1),
                );
                let _ = DataTaskToken::NpcMove(next.0.map).add_task(
                    world,
                    storage,
                    &MovePacket::new(*entity, next.0, false, true, next.1),
                );
            } else {
                npc_swap_pos(world, storage, entity, next.0);
                let _ = DataTaskToken::NpcMove(next.0.map).add_task(
                    world,
                    storage,
                    &MovePacket::new(*entity, next.0, false, false, next.1),
                );
            }
        }
    }
}
