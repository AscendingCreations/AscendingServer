use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    maps::*,
    npcs::*,
    players::*,
};
use chrono::Duration;
use rand::{thread_rng, Rng};

pub async fn check_target(world: &GameWorld, entity: &Entity) -> Result<()> {
    match world.get_or_err::<Target>(entity).await?.target_type {
        EntityType::Player(i, accid) => {
            let id = {
                let lock = world.read().await;
                let id = lock.get::<&Account>(i.0)?.id;
                id
            };

            if world.contains(&i).await
                && world.get_or_err::<DeathType>(&i).await?.is_alive()
                && id == accid
            {
                return Ok(());
            }

            {
                let lock = world.write().await;
                *lock.get::<&mut Target>(entity.0)? = Target::default();
            }

            npc_clear_move_path(world, entity).await?;
            Ok(())
        }
        EntityType::Npc(i) => {
            if is_npc_same(entity, &i) {
                return Ok(()); //targeting ourselve maybe for healing lets continue.
            }

            if world.contains(&i).await && world.get_or_err::<DeathType>(&i).await?.is_alive() {
                return Ok(());
            }

            {
                let lock = world.write().await;
                *lock.get::<&mut Target>(entity.0)? = Target::default();
            }

            npc_clear_move_path(world, entity).await?;
            Ok(())
        }
        _ => Ok(()),
    }
}

pub async fn targeting(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    base: &NpcData,
) -> Result<()> {
    // Check if we have a current Target and that they are Alive.
    // This way we dont need to change the target if we have one.
    check_target(world, entity).await?;

    if world.get_or_err::<Target>(entity).await?.target_type != EntityType::None {
        if (base.target_auto_switch
            && world.get_or_err::<Target>(entity).await?.target_timer
                < *storage.gettick.read().await)
            || (base.target_range_dropout
                && world
                    .get_or_err::<Position>(entity)
                    .await?
                    .checkdistance(world.get_or_err::<Target>(entity).await?.target_pos)
                    > base.sight)
        {
            {
                let lock = world.write().await;
                *lock.get::<&mut Target>(entity.0)? = Target::default();
            }
            npc_clear_move_path(world, entity).await?;
        } else {
            return Ok(());
        }
    }

    if !base.is_agressive() {
        return Ok(());
    }

    let map_range = get_maps_in_range(
        storage,
        &world.get_or_err::<Position>(entity).await?,
        base.sight,
    );
    let valid_map_data = map_range
        .iter()
        .filter_map(|map_pos| map_pos.get())
        .filter_map(|i| storage.maps.get(&i));

    for map_data_ref in valid_map_data {
        let map_data = map_data_ref.read().await;

        for x in &map_data.players {
            let accid = if world.contains(x).await {
                let lock = world.read().await;
                let id = lock.get::<&Account>(x.0)?.id;
                id
            } else {
                continue;
            };

            if npc_targeting(world, storage, entity, base, EntityType::Player(*x, accid)).await? {
                return Ok(());
            }
        }

        if base.has_enemies {
            for x in &map_data.npcs {
                if is_npc_same(x, entity) {
                    continue;
                }

                if npc_targeting(world, storage, entity, base, EntityType::Npc(*x)).await? {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

pub async fn try_target_entity(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    entitytype: EntityType,
) -> Result<()> {
    let target = world.get_or_err::<Target>(entity).await?;
    let pos = world.get_or_err::<Position>(entity).await?;
    let new_target = match entitytype {
        EntityType::Player(id, _) | EntityType::Npc(id) => match target.target_type {
            EntityType::Npc(oldid) | EntityType::Player(oldid, _) => oldid == id,
            _ => false,
        },
        _ => false,
    };

    let cantarget = match target.target_type {
        EntityType::Npc(id) | EntityType::Player(id, _) => {
            if world.contains(&id).await {
                let rng = {
                    let mut rng = thread_rng();
                    rng.gen_range(0..2)
                };

                if rng == 1 && new_target {
                    true
                } else {
                    let target_pos = world.get_or_err::<Position>(&id).await?;
                    let deathtype = world.get_or_err::<DeathType>(&id).await?;
                    !can_target(pos, target_pos, deathtype, 1)
                }
            } else {
                true
            }
        }
        _ => true,
    };

    let npc_index = world.get_or_default::<NpcIndex>(entity).await.0;
    let npc_base = storage.bases.npcs.get(npc_index as usize);

    if let Some(base) = npc_base
        && cantarget
    {
        let entity_copy = entitytype;
        match entitytype {
            EntityType::Npc(id) | EntityType::Player(id, _) => {
                if world.contains(&id).await {
                    let target_pos = world.get_or_err::<Position>(&id).await?;
                    let deathtype = world.get_or_err::<DeathType>(&id).await?;
                    if can_target(pos, target_pos, deathtype, 1) {
                        let lock = world.write().await;
                        lock.get::<&mut Target>(entity.0)?.target_pos = target_pos;
                        lock.get::<&mut Target>(entity.0)?.target_type = entity_copy;
                        lock.get::<&mut Target>(entity.0)?.target_timer =
                            *storage.gettick.read().await
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

pub async fn update_target_pos(world: &GameWorld, entity: &Entity) -> Result<Target> {
    if !world.contains(entity).await {
        return Ok(Target::default());
    }

    let pos = world.get_or_err::<Position>(entity).await?;
    let mut target = world.get_or_err::<Target>(entity).await?;
    let target_type = target.target_type;

    match target_type {
        EntityType::Npc(id) | EntityType::Player(id, _) => {
            if world.contains(&id).await {
                let target_pos = world.get_or_err::<Position>(&id).await?;
                let deathtype = world.get_or_err::<DeathType>(&id).await?;

                if check_surrounding(pos.map, target_pos.map, true) == MapPos::None
                    || !deathtype.is_alive()
                {
                    target = Target::default();
                } else {
                    target.target_pos = target_pos;
                }
            } else {
                target = Target::default();
            }
        }
        _ => {}
    }

    let lock = world.write().await;
    *lock.get::<&mut Target>(entity.0)? = target;

    Ok(target)
}

pub async fn npc_targeting(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    base: &NpcData,
    entitytype: EntityType,
) -> Result<bool> {
    let (pos, _) = match entitytype {
        EntityType::Player(i, accid) => {
            if world.contains(&i).await {
                let id = {
                    let lock = world.read().await;
                    let id = lock.get::<&Account>(i.0)?.id;
                    id
                };

                if world.get_or_err::<DeathType>(&i).await?.is_alive() && id == accid {
                    let check = check_surrounding(
                        world.get_or_err::<Position>(entity).await?.map,
                        world.get_or_err::<Position>(&i).await?.map,
                        true,
                    );
                    let pos = world
                        .get_or_err::<Position>(&i)
                        .await?
                        .map_offset(check.into());
                    let dir = world.get_or_err::<Dir>(&i).await?.0;
                    (pos, dir)
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        EntityType::Npc(i) => {
            if world.contains(&i).await {
                //let newbase = &storage.bases.npcs[world.get_or_err::<NpcIndex>(&i)?.0 as usize];
                let mut is_enemy = false;

                if base.has_enemies {
                    let npc_index = world.get_or_default::<NpcIndex>(&i).await.0;
                    is_enemy = base.enemies.iter().any(|&x| npc_index == x);
                }

                if world.get_or_err::<DeathType>(&i).await?.is_alive() && is_enemy {
                    let check = check_surrounding(
                        world.get_or_err::<Position>(entity).await?.map,
                        world.get_or_err::<Position>(&i).await?.map,
                        true,
                    );
                    let pos = world
                        .get_or_err::<Position>(&i)
                        .await?
                        .map_offset(check.into());
                    let dir = world.get_or_err::<Dir>(&i).await?.0;
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

    let distance = world
        .get_or_err::<Position>(entity)
        .await?
        .checkdistance(pos);

    if distance > base.sight {
        return Ok(false);
    }

    let lock = world.write().await;
    lock.get::<&mut Target>(entity.0)?.target_pos = pos;
    lock.get::<&mut Target>(entity.0)?.target_type = entitytype;
    lock.get::<&mut Target>(entity.0)?.target_timer = *storage.gettick.read().await
        + Duration::try_milliseconds(base.target_auto_switch_chance).unwrap_or_default();
    lock.get::<&mut AttackTimer>(entity.0)?.0 = *storage.gettick.read().await
        + Duration::try_milliseconds(base.attack_wait).unwrap_or_default();

    Ok(true)
}
