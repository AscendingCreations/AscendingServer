use crate::{gametypes::*, maps::*, npcs::*, players::*, tasks::*, GlobalKey};
use rand::{thread_rng, Rng};

#[inline(always)]
pub async fn damage_npc(store: &mut MapActorStore, key: GlobalKey, damage: i32) -> Result<()> {
    if let Some(npc) = store.npcs.get_mut(&key) {
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
    npc_info: NpcInfo,
    npc_mode: NpcMode,
    target: Target,
    cast_type: NpcCastType,
) -> Result<NpcStage> {
    let data = match target {
        Target::Player {
            key,
            uid: _,
            position: _,
        } => {
            let data = store
                .players
                .get(&key)
                .map(|player| (player.is_using.inuse(), player.position, player.death));

            if npc_info.data.can_attack_player || matches!(npc_mode, NpcMode::Pet | NpcMode::Summon)
            {
                data
            } else {
                None
            }
        }
        Target::Npc { key, position: _ } => {
            let data = store
                .npcs
                .get(&key)
                .map(|npc| (npc.index, npc.position, npc.death));

            if let Some((index, position, death)) = data {
                if npc_info.data.has_enemies
                    && cast_type == NpcCastType::Enemy
                    && npc_info.data.enemies.iter().any(|e| *e == index)
                {
                    Some((false, position, death))
                } else {
                    None
                }
            } else {
                None
            }
        }
        Target::Map(_) => return Ok(NpcStage::None),
        Target::None
        | Target::MapItem {
            key: _,
            position: _,
        } => return Ok(NpcStage::None),
    };

    if let Some((inuse, position, death)) = data {
        if !inuse {
            if let Some(dir) = npc_info.position.checkdirection(position) {
                if !map.is_dir_blocked(npc_info.position, dir as u8)
                    && entity_cast_check(npc_info.position, position, death, npc_info.data.range)
                {
                    return Ok(NpcStage::Combat(CombatStage::None));
                }
            }
        }
    }

    Ok(NpcStage::None)
}

pub async fn behaviour_check(store: &mut MapActorStore, npc_info: NpcInfo) -> Result<NpcStage> {
    match npc_info.data.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            if let Some(npc) = store.npcs.get(&npc_info.key) {
                Ok(CombatStage::check_target(
                    npc_info,
                    npc.mode,
                    npc.target.target,
                    NpcCastType::Enemy,
                ))
            } else {
                Ok(NpcStage::None)
            }
        }
        AIBehavior::Healer | AIBehavior::Friendly => Ok(NpcStage::None),
    }
}

pub async fn can_attack_npc(
    map: &mut MapActor,
    store: &mut MapActorStore,
    key: GlobalKey,
) -> Result<bool> {
    let index = if let Some(npc) = store.npcs.get(&key) {
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
            if let Some(player) = store.players.get(&player_key).cloned() {
                let damage =
                    npc_combat_damage(store, key, player_key, base, WorldEntityType::Player)
                        .await?;
                //damage_player(map, store, player_key, damage).await?;
                DataTaskToken::Damage.add_task(
                    map,
                    damage_packet(key, damage as u16, player.position, true)?,
                )?;
                DataTaskToken::Attack.add_task(map, attack_packet(key)?)?;

                if player.vital[0] > 0 {
                    DataTaskToken::Vitals.add_task(
                        map,
                        vitals_packet(player_key, player.vital, player.vitalmax)?,
                    )?;
                } else {
                    //remove_all_npc_target(world, &player_key).await?;
                    //kill_player(world, storage, &player_key).await?;
                    todo!()
                }
            }
        }
        Target::Npc(npc_key, _map_pos) => {
            if let Some(npc) = store.npcs.get(&npc_key).cloned() {
                let damage =
                    npc_combat_damage(store, key, npc_key, base, WorldEntityType::Npc).await?;
                damage_npc(store, npc_key, damage).await?;

                DataTaskToken::Damage
                    .add_task(map, damage_packet(key, damage as u16, npc.position, true)?)?;
                DataTaskToken::Attack.add_task(map, attack_packet(key)?)?;

                if npc.vital[0] > 0 {
                    DataTaskToken::Vitals
                        .add_task(map, vitals_packet(npc_key, npc.vital, npc.vitalmax)?)?;
                    //try_target_entity(world, storage, npc_key, Target::Npc(*key)).await?;
                } else {
                    kill_npc(map, store, npc_key).await?;
                }
            }
        }
        Target::Map(_) | Target::None | Target::MapItem(_, _) => {}
    }

    Ok(())
}

pub async fn npc_combat_damage(
    store: &mut MapActorStore,
    key: GlobalKey,
    enemy_key: GlobalKey,
    base: &NpcData,
    entity_type: WorldEntityType,
) -> Result<i32> {
    let def = match entity_type {
        WorldEntityType::Player => {
            let (def, level) = if let Some(player) = store.players.get(&enemy_key) {
                (player.defense, player.level)
            } else {
                return Ok(0);
            };

            def //+ player_get_armor_defense(map, store, enemy_key).await?.0 as u32
                + level.saturating_div(5) as u32
        }
        WorldEntityType::Npc => {
            if let Some(npc) = store.npcs.get(&enemy_key) {
                npc.defense
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
        let mut damage = npc.damage.saturating_sub(def / offset).max(base.mindamage);
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
    let (npc_base, position) = if let Some(npc) = store.npcs.get(&key) {
        (
            map.storage.bases.npcs[npc.index as usize].clone(),
            npc.position,
        )
    } else {
        return Ok(());
    };

    let mut rng = thread_rng();

    if npc_base.max_shares > 0 {
        let r = rng.gen_range(0..npc_base.max_shares);
        if let Some(&drop_id) = npc_base.drop_ranges.get(&r) {
            //do item drops here for this drop.
            if let Some(drop_data) = npc_base.drops.get(drop_id) {
                for drop in drop_data.items.iter() {
                    if drop.item > 0 && {
                        let (drop, _wait_amount) = try_drop_item(
                            map,
                            store,
                            DropItem {
                                index: drop.item,
                                amount: drop.amount as u16,
                                pos: position,
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

    if let Some(npc) = store.npcs.get_mut(&key) {
        npc.death = Death::Dead;
    }

    Ok(())
}
