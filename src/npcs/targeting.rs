use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, players::*};
use chrono::Duration;
use hecs::World;

pub fn targeting(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    base: &NpcData,
) -> Result<()> {
    // Check if we have a current Target and that they are Alive.
    // This way we dont need to change the target if we have one.
    (|| -> Result<()> {
        match world.get_or_err::<Target>(entity)?.targettype {
            EntityType::Player(i, accid) => {
                if world.contains(i.0)
                    && world.get_or_err::<DeathType>(&i)?.is_alive()
                    && world.get::<&Account>(i.0)?.id == accid
                {
                    return Ok(());
                }

                *world.get::<&mut Target>(entity.0)? = Target::default();
                Ok(())
            }
            EntityType::Npc(i) => {
                if is_npc_same(entity, &i) {
                    return Ok(()); //targeting ourselve maybe for healing lets continue.
                }

                if world.contains(i.0) && world.get_or_err::<DeathType>(&i)?.is_alive() {
                    return Ok(());
                }

                *world.get::<&mut Target>(entity.0)? = Target::default();
                Ok(())
            }
            _ => Ok(()),
        }
    })()?;

    if world.get_or_err::<Target>(entity)?.targettype != EntityType::None {
        if (base.target_auto_switch
            && world.get_or_err::<Target>(entity)?.targettimer < *storage.gettick.borrow())
            || (base.target_range_dropout
                && world
                    .get_or_err::<Position>(entity)?
                    .checkdistance(world.get_or_err::<Target>(entity)?.targetpos)
                    > base.sight)
        {
            *world.get::<&mut Target>(entity.0)? = Target::default();
        } else {
            return Ok(());
        }
    }

    if !base.is_agressive() {
        return Ok(());
    }

    let map_range = get_maps_in_range(storage, &world.get_or_err::<Position>(entity)?, base.sight);
    let valid_map_data = map_range
        .iter()
        .filter_map(|map_pos| map_pos.get())
        .filter_map(|i| storage.maps.get(&i));

    for map_data_ref in valid_map_data {
        let map_data = map_data_ref.borrow();

        for x in &map_data.players {
            let accid = if world.contains(x.0) {
                world.get::<&Account>(x.0)?.id
            } else {
                continue;
            };

            if npc_targeting(world, storage, entity, base, EntityType::Player(*x, accid))? {
                return Ok(());
            }
        }

        if base.has_enemies {
            for x in &map_data.npcs {
                if is_npc_same(x, entity) {
                    continue;
                }

                if npc_targeting(world, storage, entity, base, EntityType::Npc(*x))? {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

pub fn try_target_entity(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    entitytype: EntityType,
) -> Result<()> {
    let target = world.get_or_err::<Target>(entity)?;
    let pos = world.get_or_err::<Position>(entity)?;

    let cantarget = match target.targettype {
        EntityType::Npc(id) | EntityType::Player(id, _) => {
            if world.contains(id.0) {
                let target_pos = world.get_or_err::<Position>(&id)?;
                let deathtype = world.get_or_err::<DeathType>(&id)?;
                !can_target(pos, target_pos, deathtype, 1)
            } else {
                true
            }
        }
        _ => true,
    };

    let npc_index = world.get_or_default::<NpcIndex>(entity).0;
    let npc_base = storage.bases.npcs.get(npc_index as usize);

    if let Some(base) = npc_base
        && cantarget
    {
        let entity_copy = entitytype;
        match entitytype {
            EntityType::Npc(id) | EntityType::Player(id, _) => {
                if world.contains(id.0) {
                    let target_pos = world.get_or_err::<Position>(&id)?;
                    let deathtype = world.get_or_err::<DeathType>(&id)?;
                    if can_target(pos, target_pos, deathtype, 1) {
                        world.get::<&mut Target>(entity.0)?.targetpos = target_pos;
                        world.get::<&mut Target>(entity.0)?.targettype = entity_copy;
                        world.get::<&mut Target>(entity.0)?.targettimer = *storage.gettick.borrow()
                            + Duration::try_milliseconds(base.target_auto_switch_chance)
                                .unwrap_or_default();
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn update_target_pos(world: &mut World, entity: &Entity) -> Result<()> {
    if !world.contains(entity.0) {
        return Ok(());
    }

    let pos = world.get_or_err::<Position>(entity)?;

    match world.get_or_err::<Target>(entity)?.targettype {
        EntityType::Npc(id) | EntityType::Player(id, _) => {
            if world.contains(id.0) {
                let target_pos = world.get_or_err::<Position>(&id)?;
                let deathtype = world.get_or_err::<DeathType>(&id)?;

                if check_surrounding(pos.map, target_pos.map, true) == MapPos::None
                    || !deathtype.is_alive()
                {
                    *world.get::<&mut Target>(entity.0)? = Target::default();
                } else {
                    world.get::<&mut Target>(entity.0)?.targetpos = target_pos;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

pub fn npc_targeting(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    base: &NpcData,
    entitytype: EntityType,
) -> Result<bool> {
    let (pos, _) = match entitytype {
        EntityType::Player(i, accid) => {
            if world.contains(i.0) {
                if world.get_or_err::<DeathType>(&i)?.is_alive()
                    && world.get::<&Account>(entity.0)?.id == accid
                {
                    let check = check_surrounding(
                        world.get_or_err::<Position>(entity)?.map,
                        world.get_or_err::<Position>(&i)?.map,
                        true,
                    );
                    let pos = world.get_or_err::<Position>(&i)?.map_offset(check.into());
                    let dir = world.get_or_err::<Dir>(&i)?.0;
                    (pos, dir)
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        EntityType::Npc(i) => {
            if world.contains(i.0) {
                let newbase = &storage.bases.npcs[world.get_or_err::<NpcIndex>(&i)?.0 as usize];
                let mut is_enemy = false;

                if newbase.has_enemies {
                    is_enemy = newbase
                        .enemies
                        .iter()
                        .any(|&x| world.get_or_default::<NpcIndex>(&i).0 == x);
                }

                if world.get_or_err::<DeathType>(&i)?.is_alive() || !is_enemy {
                    let check = check_surrounding(
                        world.get_or_err::<Position>(entity)?.map,
                        world.get_or_err::<Position>(&i)?.map,
                        true,
                    );
                    let pos = world.get_or_err::<Position>(&i)?.map_offset(check.into());
                    let dir = world.get_or_err::<Dir>(&i)?.0;
                    (pos, dir)
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => return Ok(false),
    };

    if world.get_or_err::<Position>(entity)?.checkdistance(pos) <= base.sight {
        return Ok(false);
    }

    world.get::<&mut Target>(entity.0)?.targetpos = pos;
    world.get::<&mut Target>(entity.0)?.targettype = entitytype;
    world.get::<&mut Target>(entity.0)?.targettimer = *storage.gettick.borrow()
        + Duration::try_milliseconds(base.target_auto_switch_chance).unwrap_or_default();
    world.get::<&mut AttackTimer>(entity.0)?.0 = *storage.gettick.borrow()
        + Duration::try_milliseconds(base.attack_wait).unwrap_or_default();

    Ok(true)
}
