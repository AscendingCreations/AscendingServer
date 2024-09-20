use std::borrow::Borrow;

use crate::{gametypes::*, maps::*, npcs::*, players::*, tasks::*, GlobalKey};
use rand::{thread_rng, Rng};

#[inline(always)]
pub fn damage_npc(store: &mut MapActorStore, key: GlobalKey, damage: i32) -> Result<()> {
    if let Some(npc) = store.npcs.get(&key) {
        let mut npc = npc.borrow_mut();
        npc.vital[VitalTypes::Hp as usize] =
            npc.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }

    Ok(())
}

fn entity_cast_check(
    caster_pos: Position,
    target_pos: Position,
    target_death: Death,
    range: i32,
) -> bool {
    let check = check_surrounding(caster_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    range >= caster_pos.checkdistance(pos) && target_death.is_alive()
}

pub fn try_cast(
    map: &mut MapActor,
    store: &mut MapActorStore,
    caster_key: GlobalKey,
    base: &NpcData,
    target: Target,
    range: i32,
    casttype: NpcCastType,
) -> Result<bool> {
    if let Some(npc) = store.npcs.get(&caster_key) {
        let caster_pos = npc.borrow().position;
        let npc_mode = npc.borrow().mode;

        match target {
            Target::Player(key, _accid, _map_pos) => {
                let data = if let Some(player) = store.players.get(&key) {
                    Some((
                        player.borrow().is_using.inuse(),
                        player.borrow().position,
                        player.borrow().death,
                    ))
                } else if let Some(player) = store.player_info.get(&key) {
                    Some((player.is_using.inuse(), player.position, player.death))
                } else {
                    None
                };

                if let Some((inuse, position, death)) = data {
                    if (base.can_attack_player
                        || matches!(npc_mode, NpcMode::Pet | NpcMode::Summon))
                        && !inuse
                    {
                        if let Some(dir) = caster_pos.checkdirection(position) {
                            if map.is_dir_blocked(caster_pos, dir as u8) {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }

                        return Ok(entity_cast_check(caster_pos, position, death, range));
                    }
                }
            }
            Target::Npc(key, _map_pos) => {
                let data = if let Some(npc) = store.npcs.get(&key) {
                    Some((
                        npc.borrow().index,
                        npc.borrow().position,
                        npc.borrow().death,
                    ))
                } else if let Some(npc) = store.npc_info.get(&key) {
                    Some((npc.index, npc.position, npc.death))
                } else {
                    None
                };

                if let Some((index, position, death)) = data {
                    if base.has_enemies
                        && casttype == NpcCastType::Enemy
                        && base.enemies.iter().any(|e| *e == index)
                    {
                        if let Some(dir) = caster_pos.checkdirection(position) {
                            if map.is_dir_blocked(caster_pos, dir as u8) {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }

                        return Ok(entity_cast_check(caster_pos, position, death, range));
                    }
                }
            }
            Target::Map(_) | Target::None | Target::MapItem(_, _) => {}
        }
    }

    Ok(false)
}

pub enum NpcPackets {
    NpcCast {},
}

pub async fn npc_cast(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    base: &NpcData,
) -> Result<Target> {
    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            let target_type = if let Some(npc) = store.npcs.get(&key) {
                npc.borrow().target.target_type
            } else {
                return Ok(Target::None);
            };

            if try_cast(
                map,
                store,
                key,
                base,
                target_type,
                base.range,
                NpcCastType::Enemy,
            )? {
                return Ok(target_type);
            }

            Ok(Target::None)
        }
        AIBehavior::Healer | AIBehavior::Friendly => Ok(Target::None),
    }
}

pub async fn can_attack_npc(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
) -> Result<bool> {
    let index = if let Some(npc) = store.npcs.get(&key) {
        npc.borrow().index
    } else if let Some(npc) = store.npc_info.get(&key) {
        npc.index
    } else {
        return Ok(false);
    };

    let base = &map.storage.bases.npcs[index as usize];

    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => Ok(true),
        AIBehavior::Healer | AIBehavior::Friendly => Ok(false),
    }
}

pub async fn npc_combat(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
    base: &NpcData,
) -> Result<()> {
    match npc_cast(map, store, key, base).await? {
        Target::Player(player_key, _accid, _map_pos) => {
            if store.players.contains_key(&player_key)
                || store.player_info.contains_key(&player_key)
            {
                let damage = npc_combat_damage(world, storage, entity, &i, base).await?;
                damage_player(world, &i, damage).await?;
                DataTaskToken::Damage(world.get_or_default::<Position>(entity).await.map)
                    .add_task(
                        storage,
                        damage_packet(
                            *entity,
                            damage as u16,
                            world.get_or_default::<Position>(&i).await,
                            true,
                        )?,
                    )
                    .await?;
                DataTaskToken::Attack(world.get_or_default::<Position>(entity).await.map)
                    .add_task(storage, attack_packet(*entity)?)
                    .await?;
                let vitals = world.get_or_err::<Vitals>(&i).await?;
                if vitals.vital[0] > 0 {
                    DataTaskToken::Vitals(world.get_or_default::<Position>(entity).await.map)
                        .add_task(storage, {
                            let vitals = world.get_or_err::<Vitals>(&i).await?;

                            vitals_packet(i, vitals.vital, vitals.vitalmax)?
                        })
                        .await?;
                } else {
                    remove_all_npc_target(world, &i).await?;
                    kill_player(world, storage, &i).await?;
                }
            }
        }
        Target::Npc(npc_key, _map_pos) => {
            if world.contains(&i).await {
                let damage = npc_combat_damage(world, storage, entity, &i, base).await?;
                damage_npc(world, &i, damage).await?;

                DataTaskToken::Damage(world.get_or_default::<Position>(entity).await.map)
                    .add_task(
                        storage,
                        damage_packet(
                            *entity,
                            damage as u16,
                            world.get_or_default::<Position>(&i).await,
                            true,
                        )?,
                    )
                    .await?;
                DataTaskToken::Attack(world.get_or_default::<Position>(entity).await.map)
                    .add_task(storage, attack_packet(*entity)?)
                    .await?;

                let vitals = world.get_or_err::<Vitals>(&i).await?;
                if vitals.vital[0] > 0 {
                    DataTaskToken::Vitals(world.get_or_default::<Position>(entity).await.map)
                        .add_task(storage, {
                            vitals_packet(i, vitals.vital, vitals.vitalmax)?
                        })
                        .await?;
                    try_target_entity(world, storage, &i, Target::Npc(*entity)).await?;
                } else {
                    kill_npc(world, storage, &i).await?;
                }
            }
        }
        Target::Map(_) | Target::None | Target::MapItem(_) => {}
    }

    Ok(())
}

pub async fn npc_combat_damage(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: &GlobalKey,
    enemy_key: &GlobalKey,
    base: &NpcData,
    entity_type: WorldEntityType,
) -> Result<i32> {
    let def = match entity_type {
        WorldEntityType::Player => {
            let def = if let Some(player) = store.players.get(&enemy_key) {
                player.defense
            } else if let Some(player) = store.player_info.get(&enemy_key) {
                player.defense
            };
        }
        WorldEntityType::Npc => todo!(),
        _ => return Ok(0),
    };

    if world.get_or_err::<WorldGlobalKeyType>(enemy_entity).await? == WorldGlobalKeyType::Player {
        world.get_or_err::<Physical>(enemy_entity).await?.defense
            + player_get_armor_defense(world, storage, entity).await?.0 as u32
            + world
                .get_or_err::<Level>(enemy_entity)
                .await?
                .0
                .saturating_div(5) as u32
    } else {
        world.get_or_err::<Physical>(enemy_entity).await?.defense
    };

    let lock = world.read().await;
    let data = lock.entity(entity.0)?;
    let edata = lock.entity(enemy_entity.0)?;

    let offset = if edata.get_or_err::<WorldGlobalKeyType>()? == WorldGlobalKeyType::Player {
        4
    } else {
        2
    };

    let mut damage = data
        .get_or_err::<Physical>()?
        .damage
        .saturating_sub(def / offset)
        .max(base.mindamage);
    let mut rng = thread_rng();

    //set to max before we set to max i32 just in case. Order matters here.
    if damage > base.maxdamage {
        damage = base.maxdamage;
    }

    //protect from accidental heals due to u32 to i32 conversion.
    if damage >= i32::MAX as u32 {
        damage = (i32::MAX - 1) as u32;
    }

    //lets randomize are damage range so every attack doesnt always deal the same damage.
    damage = rng.gen_range(base.mindamage..=damage);

    //lets randomize to see if we do want to deal 1 damage if Defense is to high.
    if damage == 0 {
        let mut rng = thread_rng();
        damage = rng.gen_range(0..=1);
    }

    Ok(damage as i32)
}

pub async fn kill_npc(
    store: &mut MapActorStore,
    storage: &GameStore,
    key: &GlobalKey,
) -> Result<()> {
    let npc_index = world.get_or_err::<NpcIndex>(entity).await?.0;
    let npc_pos = world.get_or_err::<Position>(entity).await?;
    let npcbase = storage.bases.npcs[npc_index as usize].borrow();

    let mut rng = thread_rng();

    if npcbase.max_shares > 0 {
        let r = rng.gen_range(0..npcbase.max_shares);
        if let Some(&drop_id) = npcbase.drop_ranges.get(&r) {
            //do item drops here for this drop.
            if let Some(drop_data) = npcbase.drops.get(drop_id) {
                for drop in drop_data.items.iter() {
                    if drop.item > 0
                        && !try_drop_item(
                            world,
                            storage,
                            DropItem {
                                index: drop.item,
                                amount: drop.amount as u16,
                                pos: npc_pos,
                            },
                            None,
                            None,
                            None,
                        )
                        .await?
                    {
                        break;
                    }
                }
            }
        }
    }

    let lock = world.write().await;
    *lock.get::<&mut Death>(entity.0)? = Death::Dead;
    Ok(())
}
