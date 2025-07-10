use std::borrow::Borrow;

use crate::{
    containers::{DeathType, Entity, EntityKind, GlobalKey, NpcMode, Storage, World},
    gametypes::*,
    maps::*,
    npcs::*,
    players::*,
    tasks::*,
};
use rand::{Rng, rng};

pub fn damage_npc(world: &mut World, entity: GlobalKey, damage: i32) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        n_data.combat.vitals.vital[VitalTypes::Hp as usize] = n_data.combat.vitals.vital
            [VitalTypes::Hp as usize]
            .saturating_sub(damage)
            .max(0);
    }
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

pub fn try_cast(
    world: &mut World,
    storage: &Storage,
    caster: GlobalKey,
    base: &NpcData,
    target: GlobalKey,
    range: i32,
    casttype: NpcCastType,
) -> Result<bool> {
    if caster == target {
        return Ok(false);
    }

    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(caster) {
        let (caster_pos, caster_npc_mode) = {
            let n_data = n_data.try_lock()?;

            (n_data.movement.pos, n_data.mode)
        };

        if let Some(e_result) = world.get_opt_entity(target) {
            match e_result {
                Entity::Player(p2_data) => {
                    let (target_pos, target_death_type, is_using_type) = {
                        let p2_data = p2_data.try_lock()?;

                        (
                            p2_data.movement.pos,
                            p2_data.combat.death_type,
                            p2_data.is_using_type,
                        )
                    };

                    if (base.can_attack_player
                        || matches!(caster_npc_mode, NpcMode::Pet | NpcMode::Summon))
                        && !is_using_type.inuse()
                    {
                        if let Some(dir) = caster_pos.checkdirection(target_pos) {
                            if is_dir_blocked(storage, caster_pos, dir as u8) {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }

                        return Ok(entity_cast_check(
                            caster_pos,
                            target_pos,
                            target_death_type,
                            range,
                        ));
                    }
                }
                Entity::Npc(n2_data) => {
                    let (target_pos, target_entity_index, target_death_type) = {
                        let n2_data = n2_data.try_lock()?;

                        (
                            n2_data.movement.pos,
                            n2_data.index,
                            n2_data.combat.death_type,
                        )
                    };

                    if base.has_enemies
                        && casttype == NpcCastType::Enemy
                        && base.enemies.contains(&target_entity_index)
                    {
                        if let Some(dir) = caster_pos.checkdirection(target_pos) {
                            if is_dir_blocked(storage, caster_pos, dir as u8) {
                                return Ok(false);
                            }
                        } else {
                            return Ok(false);
                        }

                        return Ok(entity_cast_check(
                            caster_pos,
                            target_pos,
                            target_death_type,
                            range,
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(false)
}

pub fn npc_cast(
    world: &mut World,
    storage: &Storage,
    npc: GlobalKey,
    base: &NpcData,
) -> Result<Option<GlobalKey>> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(npc) {
        match base.behaviour {
            AIBehavior::Agressive
            | AIBehavior::AgressiveHealer
            | AIBehavior::ReactiveHealer
            | AIBehavior::HelpReactive
            | AIBehavior::Reactive => {
                let target = {
                    let n_data = n_data.try_lock()?;
                    n_data.combat.target
                };

                if let Some(t_entity) = target.target_entity
                    && try_cast(
                        world,
                        storage,
                        npc,
                        base,
                        t_entity,
                        base.range,
                        NpcCastType::Enemy,
                    )?
                {
                    return Ok(Some(t_entity));
                }
            }
            AIBehavior::Healer | AIBehavior::Friendly => {}
        }
    }

    Ok(None)
}

pub fn can_attack_npc(world: &mut World, storage: &Storage, npc: GlobalKey) -> Result<bool> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(npc) {
        let base = &storage.bases.npcs[n_data.try_lock()?.index as usize];

        match base.behaviour {
            AIBehavior::Agressive
            | AIBehavior::AgressiveHealer
            | AIBehavior::ReactiveHealer
            | AIBehavior::HelpReactive
            | AIBehavior::Reactive => Ok(true),
            AIBehavior::Healer | AIBehavior::Friendly => Ok(false),
        }
    } else {
        Ok(false)
    }
}

pub fn npc_combat(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    base: &NpcData,
) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let target = npc_cast(world, storage, entity, base)?;

        if let Some(t_entity) = target {
            if t_entity == entity {
                return Ok(());
            }

            let c_pos = { n_data.try_lock()?.movement.pos };

            if let Some(e_result) = world.get_opt_entity(t_entity) {
                match e_result {
                    Entity::Player(p2_data) => {
                        let damage = npc_combat_damage(world, storage, entity, t_entity, base)?;
                        damage_player(world, t_entity, damage)?;

                        let (t_pos, t_vitals) = {
                            let p2_data = p2_data.try_lock()?;

                            (p2_data.movement.pos, p2_data.combat.vitals)
                        };

                        DataTaskToken::Damage(c_pos.map).add_task(
                            storage,
                            damage_packet(entity, damage as u16, t_pos, true)?,
                        )?;
                        DataTaskToken::Attack(c_pos.map)
                            .add_task(storage, attack_packet(entity)?)?;

                        if t_vitals.vital[0] > 0 {
                            DataTaskToken::Vitals(c_pos.map).add_task(storage, {
                                vitals_packet(t_entity, t_vitals.vital, t_vitals.vitalmax)?
                            })?;
                        } else {
                            remove_all_npc_target(world, t_entity)?;
                            kill_player(world, storage, t_entity)?;
                        }
                    }
                    Entity::Npc(n2_data) => {
                        let damage = npc_combat_damage(world, storage, entity, t_entity, base)?;
                        damage_npc(world, t_entity, damage)?;

                        let (t_pos, t_vitals) = {
                            let n2_data = n2_data.try_lock()?;

                            (n2_data.movement.pos, n2_data.combat.vitals)
                        };

                        DataTaskToken::Damage(c_pos.map).add_task(
                            storage,
                            damage_packet(entity, damage as u16, t_pos, true)?,
                        )?;
                        DataTaskToken::Attack(c_pos.map)
                            .add_task(storage, attack_packet(entity)?)?;

                        if t_vitals.vital[0] > 0 {
                            DataTaskToken::Vitals(c_pos.map).add_task(storage, {
                                vitals_packet(t_entity, t_vitals.vital, t_vitals.vitalmax)?
                            })?;
                            try_target_entity(world, storage, t_entity, entity)?;
                        } else {
                            kill_npc(world, storage, t_entity)?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

pub fn npc_combat_damage(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    enemy_entity: GlobalKey,
    base: &NpcData,
) -> Result<i32> {
    if entity == enemy_entity {
        return Ok(0);
    }

    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let n_data = n_data.try_lock()?;

        let enemy_kind = world.get_kind(enemy_entity)?;

        let def = if enemy_kind == EntityKind::Player {
            let armor_def = player_get_armor_defense(world, storage, entity)?.0;

            if let Some(Entity::Player(p_data)) = world.get_opt_entity(enemy_entity) {
                let p_data = p_data.try_lock()?;

                p_data.combat.physical.defense
                    + armor_def as u32
                    + p_data.combat.level.saturating_div(5) as u32
            } else {
                0
            }
        } else if let Some(Entity::Npc(n2_data)) = world.get_opt_entity(enemy_entity) {
            n2_data.try_lock()?.combat.physical.defense
        } else {
            0
        };

        let offset = if enemy_kind == EntityKind::Player {
            4
        } else {
            2
        };

        let mut damage = n_data
            .combat
            .physical
            .damage
            .saturating_sub(def / offset)
            .max(base.mindamage);
        let mut rng = rng();

        //set to max before we set to max i32 just in case. Order matters here.
        if damage > base.maxdamage {
            damage = base.maxdamage;
        }

        //protect from accidental heals due to u32 to i32 conversion.
        if damage >= i32::MAX as u32 {
            damage = (i32::MAX - 1) as u32;
        }

        //lets randomize are damage range so every attack doesnt always deal the same damage.
        damage = rng.random_range(base.mindamage..=damage);

        //lets randomize to see if we do want to deal 1 damage if Defense is to high.
        if damage == 0 {
            damage = rng.random_range(0..=1);
        }

        Ok(damage as i32)
    } else {
        Ok(0)
    }
}

pub fn kill_npc(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let (npc_index, npc_pos) = {
            let mut n_data = n_data.try_lock()?;

            n_data.combat.death_type = DeathType::Dead;

            (n_data.index, n_data.movement.pos)
        };

        let npcbase = storage.bases.npcs[npc_index as usize].borrow();

        let mut rng = rng();

        if npcbase.max_shares > 0 {
            let r = rng.random_range(0..npcbase.max_shares);
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
                            )?
                        {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
