use std::borrow::Borrow;

use crate::{gametypes::*, maps::*, npcs::*, players::*, tasks::*, GlobalKey};
use rand::{thread_rng, Rng};

#[inline(always)]
pub async fn damage_npc(store: &mut MapActorStore, key: GlobalKey, damage: i32) -> Result<()> {
    if let Some(npc) = store.npcs.get(&key) {
        let mut npc = npc.lock().await;
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

pub async fn try_cast(
    map: &mut MapActor,
    store: &mut MapActorStore,
    caster_key: GlobalKey,
    base: &NpcData,
    target: Target,
    range: i32,
    casttype: NpcCastType,
) -> Result<bool> {
    if let Some(npc) = store.npcs.get(&caster_key) {
        let caster_pos = npc.lock().await.position;
        let npc_mode = npc.lock().await.mode;

        match target {
            Target::Player(key, _accid, _map_pos) => {
                let data = if let Some(player) = store.players.get(&key) {
                    let lock = player.lock().await;
                    Some((lock.is_using.inuse(), lock.position, lock.death))
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
                    let lock = npc.lock().await;
                    Some((lock.index, lock.position, lock.death))
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
                npc.lock().await.target.target_type
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
            )
            .await?
            {
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
        npc.lock().await.index
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
            if let Some(player) = store.players.get(&player_key).cloned() {
                let damage =
                    npc_combat_damage(map, store, key, player_key, base, WorldEntityType::Player)
                        .await?;
                damage_player(map, store, player_key, damage).await?;
                DataTaskToken::Damage
                    .add_task(
                        map,
                        damage_packet(key, damage as u16, player.lock().await.position, true)?,
                    )
                    .await?;
                DataTaskToken::Attack
                    .add_task(map, attack_packet(key)?)
                    .await?;

                if player.lock().await.vital[0] > 0 {
                    DataTaskToken::Vitals
                        .add_task(map, {
                            let lock = player.lock().await;

                            vitals_packet(i, lock.vital, lock.vitalmax)?
                        })
                        .await?;
                } else {
                    remove_all_npc_target(world, &player_key).await?;
                    kill_player(world, storage, &player_key).await?;
                }
            }
        }
        Target::Npc(npc_key, _map_pos) => {
            if let Some(npc) = store.npcs.get(&npc_key).cloned() {
                let damage =
                    npc_combat_damage(map, store, key, npc_key, base, WorldEntityType::Npc).await?;
                damage_npc(store, npc_key, damage).await?;

                DataTaskToken::Damage
                    .add_task(
                        map,
                        damage_packet(key, damage as u16, npc.lock().await.position, true)?,
                    )
                    .await?;
                DataTaskToken::Attack
                    .add_task(map, attack_packet(key)?)
                    .await?;

                if npc.lock().await.vital[0] > 0 {
                    DataTaskToken::Vitals
                        .add_task(map, {
                            let lock = npc.lock().await;
                            vitals_packet(npc_key, lock.vital, lock.vitalmax)?
                        })
                        .await?;
                    try_target_entity(world, storage, npc_key, Target::Npc(*key)).await?;
                } else {
                    kill_npc(world, storage, &npc_key).await?;
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
    key: GlobalKey,
    enemy_key: GlobalKey,
    base: &NpcData,
    entity_type: WorldEntityType,
) -> Result<i32> {
    let def = match entity_type {
        WorldEntityType::Player => {
            let (def, level) = if let Some(player) = store.players.get(&enemy_key) {
                let lock = player.lock().await;
                (lock.defense, lock.level)
            } else {
                return Ok(0);
            };

            def + player_get_armor_defense(map, store, enemy_key).await?.0 as u32
                + level.saturating_div(5) as u32
        }
        WorldEntityType::Npc => {
            if let Some(npc) = store.npcs.get(&enemy_key) {
                npc.lock().await.defense
            } else {
                return Ok(0);
            }
        }
        _ => return Ok(0),
    };

    let offset = if entity_type == WorldEntityType::Player {
        4
    } else {
        2
    };

    if let Some(npc) = store.npcs.get(&key) {
        let mut damage = npc
            .lock()
            .await
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
    } else {
        Ok(0)
    }
}

pub async fn kill_npc(map: &mut MapActor, store: &mut MapActorStore, key: GlobalKey) -> Result<()> {
    let arc_npc = store.npcs.get(&key).cloned();

    if let Some(npc) = arc_npc {
        let mut npc = npc.lock().await;
        let npcbase = map.storage.bases.npcs[npc.index as usize].clone();
        let mut rng = thread_rng();

        if npcbase.max_shares > 0 {
            let r = rng.gen_range(0..npcbase.max_shares);
            if let Some(&drop_id) = npcbase.drop_ranges.get(&r) {
                //do item drops here for this drop.
                if let Some(drop_data) = npcbase.drops.get(drop_id) {
                    for drop in drop_data.items.iter() {
                        if drop.item > 0 && {
                            let (drop, _wait_amount) = try_drop_item(
                                map,
                                store,
                                DropItem {
                                    index: drop.item,
                                    amount: drop.amount as u16,
                                    pos: npc.position,
                                    player: None,
                                },
                                None,
                                None,
                                None,
                            )
                            .await?;

                            !drop
                        } {
                            break;
                        }
                    }
                }
            }
        }

        npc.death = Death::Dead;
    }
    Ok(())
}
