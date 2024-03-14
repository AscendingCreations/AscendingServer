use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, players::*};
use chrono::Duration;
use hecs::World;

pub fn targeting(world: &mut World, storage: &Storage, entity: &Entity, base: &NpcData) {
    // Check if we have a current Target and that they are Alive.
    // This way we dont need to change the target if we have one.
    (|| match world.get_or_panic::<Target>(entity).targettype {
        EntityType::Player(i, accid) => {
            if world.contains(i.0)
                && world.get_or_panic::<DeathType>(&i).is_alive()
                && world.get::<&Account>(i.0).unwrap().id == accid
            {
                return;
            }

            *world
                .get::<&mut Target>(entity.0)
                .expect("Could not find Target") = Target::default();
        }
        EntityType::Npc(i) => {
            if is_npc_same(entity, &i) {
                return; //targeting ourselve maybe for healing lets continue.
            }

            if world.contains(i.0) && world.get_or_panic::<DeathType>(&i).is_alive() {
                return;
            }

            *world
                .get::<&mut Target>(entity.0)
                .expect("Could not find Target") = Target::default();
        }
        _ => {}
    })();

    if world.get_or_panic::<Target>(entity).targettype != EntityType::None {
        if (base.target_auto_switch
            && world.get_or_panic::<Target>(entity).targettimer < *storage.gettick.borrow())
            || (base.target_range_dropout
                && world
                    .get_or_panic::<Position>(entity)
                    .checkdistance(world.get_or_panic::<Target>(entity).targetpos)
                    > base.sight)
        {
            *world
                .get::<&mut Target>(entity.0)
                .expect("Could not find Target") = Target::default();
        } else {
            return;
        }
    }

    if !base.is_agressive() {
        return;
    }

    let map_range = get_maps_in_range(storage, &world.get_or_panic::<Position>(entity), base.sight);
    let valid_map_data = map_range
        .iter()
        .filter_map(|map_pos| map_pos.get())
        .filter_map(|i| storage.maps.get(&i));

    for map_data_ref in valid_map_data {
        let map_data = map_data_ref.borrow();

        for x in &map_data.players {
            let accid = if world.contains(x.0) {
                world.get::<&Account>(x.0).unwrap().id
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

pub fn npc_targeting(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    base: &NpcData,
    entitytype: EntityType,
) -> bool {
    let (pos, _) = match entitytype {
        EntityType::Player(i, accid) => {
            if world.contains(i.0) {
                if world.get_or_panic::<DeathType>(&i).is_alive()
                    && world.get::<&Account>(entity.0).unwrap().id == accid
                {
                    let check = check_surrounding(
                        world.get_or_panic::<Position>(entity).map,
                        world.get_or_panic::<Position>(&i).map,
                        true,
                    );
                    let pos = world.get_or_panic::<Position>(&i).map_offset(check.into());
                    let dir = world.get_or_panic::<Dir>(&i).0;
                    (pos, dir)
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        EntityType::Npc(i) => {
            if world.contains(i.0) {
                let newbase = &storage.bases.npcs[world.get_or_panic::<NpcIndex>(&i).0 as usize];
                let mut is_enemy = false;

                if newbase.has_enemies {
                    is_enemy = newbase
                        .enemies
                        .iter()
                        .any(|&x| world.get_or_panic::<NpcIndex>(&i).0 == x);
                }

                if world.get_or_panic::<DeathType>(&i).is_alive() || !is_enemy {
                    let check = check_surrounding(
                        world.get_or_panic::<Position>(entity).map,
                        world.get_or_panic::<Position>(&i).map,
                        true,
                    );
                    let pos = world.get_or_panic::<Position>(&i).map_offset(check.into());
                    let dir = world.get_or_panic::<Dir>(&i).0;
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

    if world.get_or_panic::<Position>(entity).checkdistance(pos) <= base.sight {
        return false;
    }

    world
        .get::<&mut Target>(entity.0)
        .expect("Could not find Target")
        .targetpos = pos;
    world
        .get::<&mut Target>(entity.0)
        .expect("Could not find Target")
        .targettype = entitytype;
    world
        .get::<&mut Target>(entity.0)
        .expect("Could not find Target")
        .targettimer = *storage.gettick.borrow()
        + Duration::try_milliseconds(base.target_auto_switch_chance).unwrap_or_default();
    world
        .get::<&mut AttackTimer>(entity.0)
        .expect("Could not find Target")
        .0 = *storage.gettick.borrow()
        + Duration::try_milliseconds(base.attack_wait).unwrap_or_default();
    true
}
