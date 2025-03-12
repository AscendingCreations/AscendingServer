use crate::{
    containers::{DeathType, Entity, EntityKind, GlobalKey, Storage, World},
    gametypes::*,
    maps::{can_target, is_dir_blocked},
    npcs::{can_attack_npc, damage_npc, kill_npc, try_target_entity},
    players::*,
    tasks::{DataTaskToken, attack_packet, damage_packet, vitals_packet},
};
use rand::*;
use std::cmp;

#[inline]
pub fn damage_player(world: &mut World, entity: GlobalKey, damage: i32) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        p_data.combat.vitals.vital[VitalTypes::Hp as usize] = p_data.combat.vitals.vital
            [VitalTypes::Hp as usize]
            .saturating_sub(damage)
            .max(0);
    }
    Ok(())
}

pub fn get_damage_percentage(damage: u32, hp: (u32, u32)) -> f64 {
    let curhp = cmp::min(hp.0, hp.1);
    let abs_damage = cmp::min(damage, curhp) as f64;
    abs_damage / curhp as f64
}

pub fn try_player_cast(
    world: &mut World,
    storage: &Storage,
    caster: GlobalKey,
    target: GlobalKey,
) -> Result<bool> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(caster) {
        if caster == target {
            return Ok(false);
        }

        let caster_pos = {
            let p_data = p_data.try_lock()?;

            if p_data.is_using_type.inuse() {
                return Ok(false);
            }

            p_data.movement.pos
        };

        let target_kind = world.get_kind(target)?;

        let (target_pos, life) = match target_kind {
            EntityKind::Player => {
                if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target) {
                    let p2_data = p2_data.try_lock()?;

                    if p2_data.is_using_type.inuse() {
                        return Ok(false);
                    }

                    (p2_data.movement.pos, p2_data.combat.death_type)
                } else {
                    return Ok(false);
                }
            }
            EntityKind::Npc => {
                if let Some(Entity::Npc(n_data)) = world.get_opt_entity(target) {
                    let n_data = n_data.try_lock()?;
                    (n_data.movement.pos, n_data.combat.death_type)
                } else {
                    return Ok(false);
                }
            }
            _ => return Ok(false),
        };

        if let Some(dir) = caster_pos.checkdirection(target_pos) {
            if is_dir_blocked(storage, caster_pos, dir as u8) {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }

        Ok(can_target(caster_pos, target_pos, life, 1))
    } else {
        Ok(false)
    }
}

pub fn player_combat(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<bool> {
    if entity == target_entity {
        return Ok(false);
    }

    if try_player_cast(world, storage, entity, target_entity)? {
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
            let pos = {
                let p_data = p_data.try_lock()?;

                p_data.movement.pos
            };

            if let Some(e_result) = world.get_opt_entity(target_entity) {
                match e_result {
                    Entity::Player(p2_data) => {
                        let damage = player_combat_damage(world, storage, entity, target_entity)?;
                        damage_player(world, target_entity, damage)?;

                        let (t_pos, t_vitals) = {
                            let p2_data = p2_data.try_lock()?;

                            (p2_data.movement.pos, p2_data.combat.vitals)
                        };

                        DataTaskToken::Damage(pos.map).add_task(
                            storage,
                            damage_packet(target_entity, damage as u16, t_pos, true)?,
                        )?;
                        DataTaskToken::Attack(pos.map).add_task(storage, attack_packet(entity)?)?;

                        if t_vitals.vital[0] > 0 {
                            DataTaskToken::Vitals(pos.map).add_task(storage, {
                                vitals_packet(target_entity, t_vitals.vital, t_vitals.vitalmax)?
                            })?;
                        } else {
                            kill_player(world, storage, target_entity)?;
                        }
                    }
                    Entity::Npc(n2_data) => {
                        if can_attack_npc(world, storage, target_entity)? {
                            let damage =
                                player_combat_damage(world, storage, entity, target_entity)?;
                            damage_npc(world, target_entity, damage)?;

                            let (t_pos, t_vitals, npc_index, level) = {
                                let n2_data = n2_data.try_lock()?;

                                (
                                    n2_data.movement.pos,
                                    n2_data.combat.vitals,
                                    n2_data.index,
                                    n2_data.combat.level,
                                )
                            };

                            DataTaskToken::Damage(pos.map).add_task(
                                storage,
                                damage_packet(target_entity, damage as u16, t_pos, true)?,
                            )?;
                            DataTaskToken::Attack(pos.map)
                                .add_task(storage, attack_packet(entity)?)?;

                            if t_vitals.vital[0] > 0 {
                                DataTaskToken::Vitals(pos.map).add_task(storage, {
                                    vitals_packet(target_entity, t_vitals.vital, t_vitals.vitalmax)?
                                })?;

                                try_target_entity(world, storage, target_entity, entity)?;
                            } else {
                                let base = &storage.bases.npcs[npc_index as usize];
                                let exp = base.exp;

                                player_earn_exp(world, storage, entity, level, exp, 1.0)?;
                                kill_npc(world, storage, target_entity)?;
                            }
                        } else {
                            return Ok(false);
                        }
                    }
                    _ => {}
                }
            }
        }
        return Ok(true);
    }
    Ok(false)
}

pub fn player_combat_damage(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<i32> {
    if entity == target_entity {
        return Ok(0);
    }

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        let enemy_kind = world.get_kind(target_entity)?;

        let def = if enemy_kind == EntityKind::Player {
            let armor_def = player_get_armor_defense(world, storage, entity)?.0;

            if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
                let p2_data = p2_data.try_lock()?;

                p2_data.combat.physical.defense
                    + armor_def as u32
                    + p2_data.combat.level.saturating_div(5) as u32
            } else {
                0
            }
        } else if let Some(Entity::Npc(n2_data)) = world.get_opt_entity(target_entity) {
            n2_data.try_lock()?.combat.physical.defense
        } else {
            0
        };

        let offset = if enemy_kind == EntityKind::Player {
            4
        } else {
            2
        };

        let mut damage = p_data
            .combat
            .physical
            .damage
            .saturating_sub(def / offset)
            .max(1);
        let mut rng = rng();

        //protect from accidental heals due to u32 to i32 conversion.
        if damage >= i32::MAX as u32 {
            damage = (i32::MAX - 1) as u32;
        }

        //lets randomize are damage range so every attack doesnt always deal the same damage.
        damage = rng.random_range(1..=damage);

        //lets randomize to see if we do want to deal 1 damage if Defense is to high.
        if damage == 0 {
            damage = rng.random_range(0..=1);
        }

        Ok(damage as i32)
    } else {
        Ok(0)
    }
}

pub fn kill_player(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let (vitals, spawn) = {
            let mut p_data = p_data.try_lock()?;

            p_data.combat.death_type = DeathType::Dead;

            p_data.combat.vitals.vital = p_data.combat.vitals.vitalmax;
            p_data.combat.target.target_entity = None;

            (p_data.combat.vitals, p_data.movement.spawn)
        };

        player_warp(world, storage, entity, &spawn.pos, false)?;

        DataTaskToken::Vitals(spawn.pos.map).add_task(storage, {
            vitals_packet(entity, vitals.vital, vitals.vitalmax)?
        })?;
    }
    Ok(())
}
