use std::borrow::Borrow;

use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    maps::*,
    npcs::*,
    players::*,
    tasks::*,
};
use rand::{thread_rng, Rng};

#[inline(always)]
pub async fn damage_npc(world: &GameWorld, entity: &crate::Entity, damage: i32) -> Result<()> {
    let lock = world.write().await;
    let mut vital = lock.get::<&mut Vitals>(entity.0)?;
    vital.vital[VitalTypes::Hp as usize] =
        vital.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    Ok(())
}

fn entity_cast_check(
    caster_pos: Position,
    target_pos: Position,
    target_death: DeathType,
    range: i32,
) -> bool {
    let check = check_surrounding(caster_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    range >= caster_pos.checkdistance(pos) && target_death.is_alive()
}

pub async fn try_cast(
    world: &GameWorld,
    storage: &GameStore,
    caster: &Entity,
    base: &NpcData,
    target: EntityType,
    range: i32,
    casttype: NpcCastType,
) -> Result<bool> {
    if !world.contains(caster).await {
        return Ok(false);
    }

    let caster_pos = world.get_or_default::<Position>(caster).await;
    let npc_mode = world.get_or_default::<NpcMode>(caster).await;

    match target {
        EntityType::Player(i, _accid) => {
            if world.contains(&i).await
                && (base.can_attack_player || matches!(npc_mode, NpcMode::Pet | NpcMode::Summon))
                && !world.get_or_err::<IsUsingType>(&i).await?.inuse()
            {
                let target_pos = world.get_or_default::<Position>(&i).await;
                let life = world.get_or_default::<DeathType>(&i).await;

                if let Some(dir) = caster_pos.checkdirection(target_pos) {
                    if is_dir_blocked(storage, caster_pos, dir as u8).await {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }

                return Ok(entity_cast_check(caster_pos, target_pos, life, range));
            }
        }
        EntityType::Npc(i) => {
            let npc_index = world.get_or_default::<NpcIndex>(&i).await.0;
            if base.has_enemies
                && casttype == NpcCastType::Enemy
                && base.enemies.iter().any(|e| *e == npc_index)
            {
                let target_pos = world.get_or_default::<Position>(&i).await;
                let life = world.get_or_default::<DeathType>(&i).await;

                if let Some(dir) = caster_pos.checkdirection(target_pos) {
                    if is_dir_blocked(storage, caster_pos, dir as u8).await {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }

                return Ok(entity_cast_check(caster_pos, target_pos, life, range));
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
    }

    Ok(false)
}

pub async fn npc_cast(
    world: &GameWorld,
    storage: &GameStore,
    npc: &Entity,
    base: &NpcData,
) -> Result<EntityType> {
    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            let target_type = {
                let lock = world.read().await;
                let target_type = lock.get::<&Target>(npc.0).map(|t| t.target_type);
                target_type
            };

            if let Ok(targettype) = target_type {
                if try_cast(
                    world,
                    storage,
                    npc,
                    base,
                    targettype,
                    base.range,
                    NpcCastType::Enemy,
                )
                .await?
                {
                    return Ok(targettype);
                }
            }

            Ok(EntityType::None)
        }
        AIBehavior::Healer | AIBehavior::Friendly => Ok(EntityType::None),
    }
}

pub async fn can_attack_npc(world: &GameWorld, storage: &GameStore, npc: &Entity) -> Result<bool> {
    let npc_index = world.get_or_err::<NpcIndex>(npc).await?.0;
    let base = &storage.bases.npcs[npc_index as usize];

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
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    base: &NpcData,
) -> Result<()> {
    match npc_cast(world, storage, entity, base).await? {
        EntityType::Player(i, _accid) => {
            if world.contains(&i).await {
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
        EntityType::Npc(i) => {
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
                    try_target_entity(world, storage, &i, EntityType::Npc(*entity)).await?;
                } else {
                    kill_npc(world, storage, &i).await?;
                }
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
    }

    Ok(())
}

pub async fn npc_combat_damage(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    enemy_entity: &Entity,
    base: &NpcData,
) -> Result<i32> {
    let def = if world.get_or_err::<WorldEntityType>(enemy_entity).await? == WorldEntityType::Player
    {
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

    let offset = if edata.get_or_err::<WorldEntityType>()? == WorldEntityType::Player {
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

pub async fn kill_npc(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let npc_index = world.get_or_err::<NpcIndex>(entity).await?.0;
    let npc_pos = world.get_or_err::<Position>(entity).await?;
    let npcbase = storage.bases.npcs[npc_index as usize].borrow();

    if npcbase.max_shares > 0 {
        let r = {
            let mut rng = thread_rng();
            rng.gen_range(0..npcbase.max_shares)
        };

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
    *lock.get::<&mut DeathType>(entity.0)? = DeathType::Dead;
    Ok(())
}
