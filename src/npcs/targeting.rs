use crate::{containers::Storage, gametypes::*, maps::*, npcs::*};
use chrono::Duration;

pub fn targeting(world: &Storage, npc: &mut Npc, base: &NpcData) {
    // Check if we have a current Target and that they are Alive.
    // This way we dont need to change the target if we have one.
    (|| match npc.e.targettype {
        EntityType::Player(i, accid) => {
            if let Some(player) = world.players.borrow().get(i as usize) {
                if player.borrow().accid != accid || player.borrow().e.life.is_alive() {
                    return;
                }
            }

            npc.e.reset_target();
        }
        EntityType::Npc(i) => {
            if npc.is_same(i as usize) {
                return; //targeting ourselve maybe for healing lets continue.
            }

            if let Some(npc) = world.npcs.borrow().get(i as usize) {
                if npc.borrow().e.life.is_alive() {
                    return;
                }
            }

            npc.e.reset_target();
        }
        _ => {}
    })();

    if npc.e.targettype != EntityType::None {
        if (base.target_auto_switch && npc.e.targettimer < *world.gettick.borrow())
            || (base.target_range_dropout && npc.e.pos.checkdistance(npc.e.targetpos) > base.sight)
        {
            npc.e.reset_target();
        } else {
            return;
        }
    }

    if !base.is_agressive() {
        return;
    }

    let map_range = get_maps_in_range(world, &npc.e.pos, base.sight);
    let valid_map_data = map_range
        .iter()
        .filter_map(|map_pos| map_pos.get())
        .filter_map(|i| world.maps.get(&i));

    for map_data_ref in valid_map_data {
        let map_data = map_data_ref.borrow();

        for x in &map_data.players {
            let accid = if let Some(player) = world.players.borrow().get(*x) {
                player.borrow().accid
            } else {
                continue;
            };

            if npc_targeting(world, npc, base, EntityType::Player(*x as u64, accid)) {
                return;
            }
        }

        if base.has_enemies {
            for x in &map_data.npcs {
                if npc.is_same(*x) {
                    continue;
                }

                if npc_targeting(world, npc, base, EntityType::Npc(*x as u64)) {
                    return;
                }
            }
        }
    }
}

pub fn npc_targeting(world: &Storage, npc: &mut Npc, base: &NpcData, entity: EntityType) -> bool {
    let (pos, _) = match entity {
        EntityType::Player(i, accid) => {
            if let Some(target) = world.players.borrow().get(i as usize) {
                let target = target.borrow();

                if target.e.life.is_alive() && target.accid == accid {
                    let check = check_surrounding(npc.e.pos.map, target.e.pos.map, true);
                    let pos = target.e.pos.map_offset(check.into());
                    let dir = target.e.dir;
                    (pos, dir)
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        EntityType::Npc(i) => {
            if let Some(target) = world.npcs.borrow().get(i as usize) {
                let target = target.borrow();
                let newbase = &world.bases.npcs[target.num as usize];
                let mut is_enemy = false;

                if newbase.has_enemies {
                    is_enemy = newbase.enemies.iter().any(|&x| target.num == x);
                }

                if target.e.life.is_alive() || !is_enemy {
                    let check = check_surrounding(npc.e.pos.map, target.e.pos.map, true);
                    let pos = target.e.pos.map_offset(check.into());
                    let dir = target.e.dir;
                    (pos, dir)
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        EntityType::Map(_) | EntityType::None => return false,
    };

    if npc.e.pos.checkdistance(pos) <= base.sight {
        return false;
    }

    npc.e.targetpos = pos;
    npc.e.targettype = entity;
    npc.e.targettimer =
        *world.gettick.borrow() + Duration::milliseconds(base.target_auto_switch_chance);
    npc.e.attacktimer = *world.gettick.borrow() + Duration::milliseconds(base.attack_wait);
    true
}
