use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, players::*,};
use chrono::Duration;

pub fn targeting(world: &mut hecs::World, storage: &Storage, entity: &Entity, base: &NpcData) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    // Check if we have a current Target and that they are Alive.
    // This way we dont need to change the target if we have one.
    (|| match data.get::<&Target>().expect("Could not find Target").targettype {
        EntityType::Player(i, accid) => {
            if let Ok(tdata) = world.entity(i.0) {
                if tdata.get::<&DeathType>().expect("Could not find DeathType").is_alive() && 
                    tdata.get::<&Account>().expect("Could not find Account").id == accid {
                    return;
                }
            }

            if let mut target = data.get::<&mut Target>().expect("Could not find Target")
                { *target = Target::default() }
        }
        EntityType::Npc(i) => {
            if is_npc_same(entity, &i) {
                return; //targeting ourselve maybe for healing lets continue.
            }

            if let Ok(tdata) = world.entity(i.0) {
                if data.get::<&DeathType>().expect("Could not find DeathType").is_alive() {
                    return;
                }
            }

            if let mut target = data.get::<&mut Target>().expect("Could not find Target")
                { *target = Target::default() }
        }
        _ => {}
    })();

    if data.get::<&Target>().expect("Could not find Target").targettype != EntityType::None {
        if (base.target_auto_switch && 
            data.get::<&Target>().expect("Could not find Target").targettimer < *storage.gettick.borrow())
            || (base.target_range_dropout && 
                data.get::<&Position>().expect("Could not find Position")
                    .checkdistance(data.get::<&Target>().expect("Could not find Target").targetpos) > base.sight)
        {
            if let mut target = data.get::<&mut Target>().expect("Could not find Target")
                { *target = Target::default() }
        } else {
            return;
        }
    }

    if !base.is_agressive() {
        return;
    }

    let map_range = get_maps_in_range(storage, &data.get::<&Position>().expect("Could not find Position"), base.sight);
    let valid_map_data = map_range
        .iter()
        .filter_map(|map_pos| map_pos.get())
        .filter_map(|i| storage.maps.get(&i));

    for map_data_ref in valid_map_data {
        let map_data = map_data_ref.borrow();

        for x in &map_data.players {
            let accid = if let Ok(pdata) = world.entity(x.0) {
                pdata.get::<&Account>().expect("Could not find Account").id
            } else {
                continue;
            };

            if npc_targeting(world, storage, entity, base, EntityType::Player(*x, accid)) {
                return;
            }
        }

        if base.has_enemies {
            for x in &map_data.npcs {
                if is_npc_same(x, entity) {
                    continue;
                }

                if npc_targeting(world, storage, entity, base, EntityType::Npc(*x)) {
                    return;
                }
            }
        }
    }
}

pub fn npc_targeting(world: &mut hecs::World, storage: &Storage, entity: &Entity, base: &NpcData, entitytype: EntityType) -> bool {
    let data = world.entity(entity.0).expect("Could not get Entity");
    let (pos, _) = match entitytype {
        EntityType::Player(i, accid) => {
            if let Ok(tdata) = world.entity(i.0) {
                if tdata.get::<&DeathType>().expect("Could not find DeathType").is_alive() && 
                    tdata.get::<&Account>().expect("Could not find Account").id == accid {
                    let check = 
                        check_surrounding(
                            data.get::<&Position>().expect("Could not find Position").map, 
                            tdata.get::<&Position>().expect("Could not find Position").map, true);
                    let pos = tdata.get::<&Position>().expect("Could not find Position").map_offset(check.into());
                    let dir = tdata.get::<&Dir>().expect("Could not find Dir").0;
                    (pos, dir)
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        EntityType::Npc(i) => {
            if let Ok(tdata) = world.entity(i.0) {
                let newbase = 
                    &storage.bases.npcs[tdata.get::<&NpcIndex>().expect("Could not find NpcIndex").0 as usize];
                let mut is_enemy = false;

                if newbase.has_enemies {
                    is_enemy = newbase.enemies.iter()
                        .any(|&x| tdata.get::<&NpcIndex>().expect("Could not find NpcIndex").0 == x);
                }

                if tdata.get::<&DeathType>().expect("Could not find DeathType").is_alive() || !is_enemy {
                    let check = 
                        check_surrounding(
                            data.get::<&Position>().expect("Could not find Position").map, 
                            tdata.get::<&Position>().expect("Could not find Position").map, true);
                    let pos = tdata.get::<&Position>().expect("Could not find Position").map_offset(check.into());
                    let dir = tdata.get::<&Dir>().expect("Could not find Dir").0;
                    (pos, dir)
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => return false,
    };

    if data.get::<&Position>().expect("Could not find Position").checkdistance(pos) <= base.sight {
        return false;
    }

    if let mut target = data.get::<&mut Target>().expect("Could not find Target") {
        target.targetpos = pos;
        target.targettype = entitytype;
        target.targettimer =
            *storage.gettick.borrow() + Duration::milliseconds(base.target_auto_switch_chance);
    }
    if let mut attacktimer = data.get::<&mut AttackTimer>().expect("Could not find AttackTimer")
        { attacktimer.0 = *storage.gettick.borrow() + Duration::milliseconds(base.attack_wait) }
    true
}
