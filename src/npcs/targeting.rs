use crate::{gametypes::*, maps::*, npcs::*, GlobalKey};
use chrono::Duration;
use std::sync::Arc;
//use tokio::sync::Mutex;
//use rand::{thread_rng, Rng};

pub async fn check_target(
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    target: Targeting,
) -> NpcStage {
    let mut is_valid = false;

    match target.target_type {
        Target::Player(i, accid, _map_pos) => {
            if let Some(player) = store.players.get(&i) {
                if player.death.is_alive() && player.uid == accid {
                    is_valid = true;
                }
            }
        }
        Target::Npc(i, _map_pos) => {
            if let Some(npc) = store.npcs.get(&i) {
                if npc.death.is_alive() {
                    is_valid = true;
                }
            }
        }
        _ => {}
    }

    if is_valid {
        NpcStage::Targeting(TargetingStage::NpcDeTargetChance {
            key,
            npc_data,
            position,
            target,
        })
    } else {
        NpcStage::Targeting(TargetingStage::ClearTarget {
            key,
            npc_data,
            position,
        })
    }
}

pub async fn check_target_distance(
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    target: Targeting,
) -> NpcStage {
    let entity_pos = match target.target_type {
        Target::Player(i, _accid, _map_pos) => {
            if let Some(player) = store.players.get(&i) {
                Some(player.position)
            } else {
                None
            }
        }
        Target::Npc(i, _map_pos) => {
            if let Some(npc) = store.npcs.get(&i) {
                Some(npc.position)
            } else {
                None
            }
        }
        Target::Map(position) => Some(position),
        _ => None,
    };

    if let Some(entity_pos) = entity_pos {
        if position.checkdistance(entity_pos) > npc_data.sight {
            return NpcStage::Targeting(TargetingStage::ClearTarget {
                key,
                npc_data,
                position,
            });
        }
    }

    NpcStage::Targeting(TargetingStage::MoveToMovement {
        key,
        position,
        npc_data,
    })
}

pub async fn check_detargeting(
    map: &mut MapActor,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    target: Targeting,
) -> NpcStage {
    if target.target_type != Target::None {
        if npc_data.target_auto_switch && target.target_timer < map.tick {
            return NpcStage::Targeting(TargetingStage::ClearTarget {
                key,
                npc_data,
                position,
            });
        } else if npc_data.target_range_dropout {
            return NpcStage::Targeting(TargetingStage::CheckDistance {
                key,
                position,
                npc_data,
                target,
            });
        }

        return NpcStage::Targeting(TargetingStage::MoveToMovement {
            key,
            position,
            npc_data,
        });
    }

    NpcStage::Targeting(TargetingStage::GetTargetMaps {
        key,
        position,
        npc_data,
    })
}

pub async fn get_targeting_maps(
    map: &mut MapActor,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    if !npc_data.is_agressive() {
        return NpcStage::Targeting(TargetingStage::MoveToMovement {
            key,
            position,
            npc_data,
        });
    }

    let maps = get_maps_in_range(&map.storage, &position, npc_data.sight)
        .iter()
        .filter_map(|m| m.get())
        .collect();

    NpcStage::Targeting(TargetingStage::GetTargetFromMaps {
        key,
        position,
        npc_data,
        maps,
    })
}

pub async fn get_target(
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
) -> NpcStage {
    for (pkey, player) in store.players.iter_mut() {
        if let Some(Entity::Player(player)) =
            npc_targeting(position, &npc_data, Entity::Player(player)).await
        {
            let target = Target::Player(*pkey, player.uid, player.position.map);

            return NpcStage::Targeting(TargetingStage::SetTarget {
                key,
                position,
                npc_data,
                target,
                target_pos: player.position,
            });
        }
    }

    if npc_data.has_enemies {
        for (nkey, npc) in store.npcs.iter_mut() {
            if key == *nkey {
                continue;
            }

            if let Some(Entity::Npc(npc)) =
                npc_targeting(position, &npc_data, Entity::Npc(npc)).await
            {
                let target = Target::Npc(*nkey, npc.position.map);

                return NpcStage::Targeting(TargetingStage::SetTarget {
                    key,
                    position,
                    npc_data,
                    target,
                    target_pos: npc.position,
                });
            }
        }
    }

    //Tell system we found nothing to do next or something else like movement.
    NpcStage::Targeting(TargetingStage::None)
}

pub async fn set_target(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    position: Position,
    npc_data: Arc<NpcData>,
    target: Target,
    target_pos: Position,
) -> NpcStage {
    if let Some(npc) = store.npcs.get_mut(&key) {
        npc.target.target_type = target;
        npc.target.target_timer = map.tick
            + Duration::try_milliseconds(npc_data.target_auto_switch_chance).unwrap_or_default();
        npc.target.target_pos = target_pos;
        npc.attack_timer =
            map.tick + Duration::try_milliseconds(npc_data.attack_wait).unwrap_or_default();
    }

    NpcStage::Targeting(TargetingStage::MoveToMovement {
        key,
        position,
        npc_data,
    })
}

pub async fn set_stage(store: &mut MapActorStore, key: GlobalKey, stage: NpcStages) {
    if let Some(npc) = store.npcs.get_mut(&key) {
        npc.stage = stage;
    }
}

/*pub async fn try_target_entity(
    map: &mut MapActor,
    store: &mut MapActorStore,
    Key: GlobalKey,
    entitytype: Target,
) -> Result<()> {
    if let Some(npc)
    let target = world.get_or_err::<Targeting>(entity).await?;
    let pos = world.get_or_err::<Position>(entity).await?;
    let new_target = match entitytype {
        Target::Player(id, _) | Target::Npc(id) => match target.target_type {
            Target::Npc(oldid) | Target::Player(oldid, _) => oldid == id,
            _ => false,
        },
        _ => false,
    };

    let cantarget = match target.target_type {
        Target::Npc(id) | Target::Player(id, _) => {
            if world.contains(&id).await {
                let mut rng = thread_rng();

                if rng.gen_range(0..2) == 1 && new_target {
                    true
                } else {
                    let target_pos = world.get_or_err::<Position>(&id).await?;
                    let deathtype = world.get_or_err::<Death>(&id).await?;
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
            Target::Npc(id) | Target::Player(id, _) => {
                if world.contains(&id).await {
                    let target_pos = world.get_or_err::<Position>(&id).await?;
                    let deathtype = world.get_or_err::<Death>(&id).await?;
                    if can_target(pos, target_pos, deathtype, 1) {
                        let lock = world.write().await;
                        lock.get::<&mut Targeting>(entity.0)?.target_pos = target_pos;
                        lock.get::<&mut Targeting>(entity.0)?.target_type = entity_copy;
                        lock.get::<&mut Targeting>(entity.0)?.target_timer =
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

pub async fn update_target_pos(world: &GameWorld, entity: &GlobalKey) -> Result<Targeting> {
    if !world.contains(entity).await {
        return Ok(Targeting::default());
    }

    let pos = world.get_or_err::<Position>(entity).await?;
    let mut target = world.get_or_err::<Targeting>(entity).await?;
    let target_type = target.target_type;

    match target_type {
        Target::Npc(id) | Target::Player(id, _) => {
            if world.contains(&id).await {
                let target_pos = world.get_or_err::<Position>(&id).await?;
                let deathtype = world.get_or_err::<Death>(&id).await?;

                if check_surrounding(pos.map, target_pos.map, true) == MapPos::None
                    || !deathtype.is_alive()
                {
                    target = Targeting::default();
                } else {
                    target.target_pos = target_pos;
                }
            } else {
                target = Targeting::default();
            }
        }
        _ => {}
    }

    let lock = world.write().await;
    *lock.get::<&mut Targeting>(entity.0)? = target;

    Ok(target)
}*/

pub async fn npc_targeting<'a>(
    position: Position,
    npc_data: &NpcData,
    entity: Entity<'a>,
) -> Option<Entity<'a>> {
    let (pos, _) = match &entity {
        Entity::Player(player) => {
            if player.death.is_alive() {
                let check = check_surrounding(position.map, player.position.map, true);

                (player.position.map_offset(check.into()), player.dir)
            } else {
                return None;
            }
        }
        Entity::Npc(npc) => {
            if npc.death.is_alive() && npc_data.enemies.iter().any(|&x| npc.index == x) {
                let check = check_surrounding(position.map, npc.position.map, true);

                (npc.position.map_offset(check.into()), npc.dir)
            } else {
                return None;
            }
        }
        Entity::None => return None,
    };

    if position.checkdistance(pos) > npc_data.sight {
        return None;
    }

    Some(entity)
}
