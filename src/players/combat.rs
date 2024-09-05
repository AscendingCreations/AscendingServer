use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    maps::{can_target, is_dir_blocked},
    npcs::{can_attack_npc, damage_npc, kill_npc, try_target_entity, NpcIndex},
    players::*,
    tasks::{attack_packet, damage_packet, vitals_packet, DataTaskToken},
};
use rand::*;
use std::cmp;

#[inline]
pub async fn damage_player(world: &GameWorld, entity: &crate::Entity, damage: i32) -> Result<()> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Vitals>(entity.0)?;

    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize] =
            player_vital.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }

    Ok(())
}

pub fn get_damage_percentage(damage: u32, hp: (u32, u32)) -> f64 {
    let curhp = cmp::min(hp.0, hp.1);
    let abs_damage = cmp::min(damage, curhp) as f64;
    abs_damage / curhp as f64
}

pub async fn try_player_cast(
    world: &GameWorld,
    storage: &GameStore,
    caster: &Entity,
    target: &Entity,
) -> bool {
    if !world.contains(caster).await || !world.contains(target).await {
        return false;
    }

    if world.get_or_default::<IsUsingType>(caster).await.inuse()
        || world.get_or_default::<IsUsingType>(target).await.inuse()
    {
        return false;
    }

    let caster_pos = world.get_or_default::<Position>(caster).await;
    let target_pos = world.get_or_default::<Position>(target).await;
    let life = world.get_or_default::<DeathType>(target).await;

    if let Some(dir) = caster_pos.checkdirection(target_pos) {
        if is_dir_blocked(storage, caster_pos, dir as u8).await {
            return false;
        }
    } else {
        return false;
    }

    can_target(caster_pos, target_pos, life, 1)
}

pub async fn player_combat(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<bool> {
    if try_player_cast(world, storage, entity, target_entity).await {
        let world_entity_type = world.get_or_default::<WorldEntityType>(target_entity).await;
        match world_entity_type {
            WorldEntityType::Player => {
                let damage = player_combat_damage(world, storage, entity, target_entity).await?;
                damage_player(world, target_entity, damage).await?;
                DataTaskToken::Damage(world.get_or_default::<Position>(entity).await.map)
                    .add_task(
                        storage,
                        damage_packet(
                            *target_entity,
                            damage as u16,
                            world.get_or_default::<Position>(target_entity).await,
                            true,
                        )?,
                    )
                    .await?;
                DataTaskToken::Attack(world.get_or_default::<Position>(entity).await.map)
                    .add_task(storage, attack_packet(*entity)?)
                    .await?;

                let vitals = world.get_or_err::<Vitals>(target_entity).await?;
                if vitals.vital[0] > 0 {
                    DataTaskToken::Vitals(world.get_or_default::<Position>(entity).await.map)
                        .add_task(storage, {
                            vitals_packet(*target_entity, vitals.vital, vitals.vitalmax)?
                        })
                        .await?;
                } else {
                    kill_player(world, storage, target_entity).await?;
                }
            }
            WorldEntityType::Npc => {
                if can_attack_npc(world, storage, target_entity).await? {
                    let damage =
                        player_combat_damage(world, storage, entity, target_entity).await?;
                    damage_npc(world, target_entity, damage).await?;

                    DataTaskToken::Damage(world.get_or_default::<Position>(entity).await.map)
                        .add_task(
                            storage,
                            damage_packet(
                                *target_entity,
                                damage as u16,
                                world.get_or_default::<Position>(target_entity).await,
                                true,
                            )?,
                        )
                        .await?;
                    DataTaskToken::Attack(world.get_or_default::<Position>(entity).await.map)
                        .add_task(storage, attack_packet(*entity)?)
                        .await?;

                    let vitals = world.get_or_err::<Vitals>(target_entity).await?;
                    if vitals.vital[0] > 0 {
                        DataTaskToken::Vitals(world.get_or_default::<Position>(entity).await.map)
                            .add_task(storage, {
                                vitals_packet(*target_entity, vitals.vital, vitals.vitalmax)?
                            })
                            .await?;

                        let acc_id = world.cloned_get_or_default::<Account>(entity).await.id;
                        try_target_entity(
                            world,
                            storage,
                            target_entity,
                            EntityType::Player(*entity, acc_id),
                        )
                        .await?;
                    } else {
                        let npc_index = world.get_or_err::<NpcIndex>(target_entity).await?.0;
                        let base = &storage.bases.npcs[npc_index as usize];

                        let level = world.get_or_err::<Level>(target_entity).await?.0;
                        let exp = base.exp;

                        player_earn_exp(world, storage, entity, level, exp, 1.0).await?;
                        kill_npc(world, storage, target_entity).await?;
                    }
                } else {
                    return Ok(false);
                }
            }
            _ => {}
        }
        return Ok(true);
    }
    Ok(false)
}

pub async fn player_combat_damage(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<i32> {
    let def =
        if world.get_or_err::<WorldEntityType>(target_entity).await? == WorldEntityType::Player {
            world.get_or_err::<Physical>(target_entity).await?.defense
                + player_get_armor_defense(world, storage, target_entity)
                    .await?
                    .0 as u32
                + world
                    .get_or_err::<Level>(target_entity)
                    .await?
                    .0
                    .saturating_div(5) as u32
        } else {
            world.get_or_err::<Physical>(target_entity).await?.defense
        };

    let player_damage = player_get_weapon_damage(world, storage, entity).await?.0 as u32;
    let lock = world.read().await;
    let data = lock.entity(entity.0)?;
    let edata = lock.entity(target_entity.0)?;

    let offset = if edata.get_or_err::<WorldEntityType>()? == WorldEntityType::Player {
        4
    } else {
        2
    };

    let mut damage = (data.get_or_err::<Physical>()?.damage + player_damage)
        .saturating_sub(def / offset)
        .max(1);
    let mut rng = thread_rng();

    //protect from accidental heals due to u32 to i32 conversion.
    if damage >= i32::MAX as u32 {
        damage = (i32::MAX - 1) as u32;
    }

    //lets randomize are damage range so every attack doesnt always deal the same damage.
    damage = rng.gen_range(1..=damage);

    //lets randomize to see if we do want to deal 1 damage if Defense is to high.
    if damage == 0 {
        let mut rng = thread_rng();
        damage = rng.gen_range(0..=1);
    }

    Ok(damage as i32)
}

pub async fn kill_player(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    {
        let lock = world.write().await;
        if let Ok(mut vitals) = lock.get::<&mut Vitals>(entity.0) {
            vitals.vital = vitals.vitalmax;
        }
        lock.get::<&mut PlayerTarget>(entity.0)?.0 = None;
    }

    let spawn = world.get_or_err::<Spawn>(entity).await?;
    player_warp(world, storage, entity, &spawn.pos, false).await?;
    DataTaskToken::Vitals(world.get_or_default::<Position>(entity).await.map)
        .add_task(storage, {
            let vitals = world.get_or_err::<Vitals>(entity).await?;

            vitals_packet(*entity, vitals.vital, vitals.vitalmax)?
        })
        .await
    //this should not be needed anymore?
    //init_data_lists(world, storage, entity, None)
}
