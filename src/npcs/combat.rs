use crate::{gametypes::*, maps::*, npcs::*, players::*, tasks::*, GlobalKey};
use chrono::Duration;
use rand::{thread_rng, Rng};

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

pub fn check_target(
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
        Target::Map(_) => return Ok(NpcStage::None(npc_info)),
        Target::None
        | Target::MapItem {
            key: _,
            position: _,
        } => return Ok(NpcStage::None(npc_info)),
    };

    if let Some((inuse, position, death)) = data {
        if !inuse {
            if let Some(dir) = npc_info.position.checkdirection(position) {
                if !map.is_dir_blocked(npc_info.position, dir)
                    && entity_cast_check(npc_info.position, position, death, npc_info.data.range)
                {
                    return Ok(CombatStage::get_defense(npc_info, target));
                }
            }
        }
    }

    Ok(NpcStage::None(npc_info))
}

pub fn behaviour_check(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
) -> Result<NpcStage> {
    match npc_info.data.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            if let Some(npc) = store.npcs.get_mut(&npc_info.key) {
                if npc.attack_timer <= map.tick {
                    npc.attack_timer = map.tick
                        + Duration::try_milliseconds(npc_info.data.attack_wait).unwrap_or_default();

                    return Ok(CombatStage::check_target(
                        npc_info,
                        npc.mode,
                        npc.target.target,
                        NpcCastType::Enemy,
                    ));
                }
            }
        }
        AIBehavior::Healer | AIBehavior::Friendly => {}
    }

    Ok(NpcStage::None(npc_info))
}

pub fn can_attack_npc(
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

pub fn get_defense(
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    target: Target,
) -> Result<NpcStage> {
    let def = match target {
        Target::Player {
            key,
            uid: _,
            position: _,
        } => {
            let (def, level) = if let Some(player) = store.players.get(&key) {
                (player.defense, player.level)
            } else {
                return Ok(NpcStage::None(npc_info));
            };

            (def //+ player_get_armor_defense(map, store, enemy_key).await?.0 as u32
                + level.saturating_div(5) as u32)
                / 4
        }
        Target::Npc { key, position: _ } => {
            if let Some(npc) = store.npcs.get(&key) {
                npc.defense / 2
            } else {
                return Ok(NpcStage::None(npc_info));
            }
        }
        _ => return Ok(NpcStage::None(npc_info)),
    };

    Ok(CombatStage::get_damage(npc_info, def, target))
}

pub fn get_damage(
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    defense: u32,
    target: Target,
) -> Result<NpcStage> {
    if let Some(npc) = store.npcs.get(&npc_info.key) {
        let mut damage = npc
            .damage
            .saturating_sub(defense)
            .max(npc_info.data.mindamage);
        let mut rng = thread_rng();

        //set to max before we set to max i32 just in case. Order matters here.
        if damage > npc_info.data.maxdamage {
            damage = npc_info.data.maxdamage;
        }

        //protect from accidental heals due to u32 to i32 conversion.
        let mut damage = if damage >= i32::MAX as u32 {
            i32::MAX - 1
        } else {
            damage as i32
        };

        //lets randomize are damage range so every attack doesnt always deal the same damage.
        damage = rng.gen_range(npc_info.data.mindamage as i32..=damage);

        //lets randomize to see if we do want to deal 1 damage if Defense is to high.
        if damage == 0 {
            let mut rng = thread_rng();
            damage = rng.gen_range(0..=1);
        }

        Ok(CombatStage::do_damage(npc_info, damage, target))
    } else {
        Ok(NpcStage::None(npc_info))
    }
}

pub async fn do_damage(
    map: &mut MapActor,
    store: &mut MapActorStore,
    npc_info: NpcInfo,
    damage: i32,
    target: Target,
) -> Result<NpcStage> {
    match target {
        Target::Player {
            key,
            uid: _,
            position: _,
        } => {
            if let Some(player) = store.players.get(&key) {
                ////damage_player(map, store, player_key, damage).await?;
                DataTaskToken::Damage.add_task(
                    map,
                    damage_packet(key, damage as u16, player.position, true)?,
                )?;
                DataTaskToken::Attack.add_task(map, attack_packet(key)?)?;

                if player.vital[0] > 0 {
                    DataTaskToken::Vitals
                        .add_task(map, vitals_packet(key, player.vital, player.vitalmax)?)?;
                } else {
                    //remove_all_npc_target(world, &player_key).await?;
                    //kill_player(world, storage, &player_key).await?;
                    return Ok(CombatStage::remove_target(npc_info));
                }
            }
        }
        Target::Npc { key, position: _ } => {
            if let Some(npc) = store.npcs.get_mut(&key) {
                npc.damage_npc(damage);

                DataTaskToken::Damage
                    .add_task(map, damage_packet(key, damage as u16, npc.position, true)?)?;
                DataTaskToken::Attack.add_task(map, attack_packet(key)?)?;

                if npc.vital[0] > 0 {
                    DataTaskToken::Vitals
                        .add_task(map, vitals_packet(key, npc.vital, npc.vitalmax)?)?;

                    if let Some(data) = map.storage.get_npc(npc.index) {
                        npc.swap_target(map, &data, target);
                    }
                } else {
                    kill_npc(map, store, key).await?;
                    return Ok(CombatStage::remove_target(npc_info));
                }
            }
        }
        _ => {}
    }

    Ok(NpcStage::None(npc_info))
}

pub fn remove_target(store: &mut MapActorStore, npc_info: NpcInfo) -> Result<NpcStage> {
    if let Some(npc) = store.npcs.get_mut(&npc_info.key) {
        npc.target = Targeting::default();
    }

    Ok(NpcStage::None(npc_info))
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
