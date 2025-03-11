use crate::{
    containers::{Entity, GlobalKey, Storage, Target, World},
    gametypes::*,
    maps::*,
    npcs::*,
};
use chrono::Duration;
use rand::{Rng, rng};

pub fn targeting(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    base: &NpcData,
) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let target = { n_data.try_lock()?.combat.target.target_entity };

        // Check if we have a current Target and that they are Alive.
        // This way we dont need to change the target if we have one.
        (|| -> Result<()> {
            if let Some(t_entity) = target {
                if t_entity == entity {
                    return Ok(());
                }

                match world.get_opt_entity(t_entity) {
                    Some(e_result) => match e_result {
                        Entity::Player(p2_data) => {
                            {
                                if p2_data.try_lock()?.combat.death_type.is_alive() {
                                    return Ok(());
                                }

                                {
                                    n_data.try_lock()?.combat.target = Target::default();
                                }
                            }

                            npc_clear_move_path(world, entity)?;
                            Ok(())
                        }
                        Entity::Npc(n2_data) => {
                            {
                                if n2_data.try_lock()?.combat.death_type.is_alive() {
                                    return Ok(());
                                }

                                {
                                    n_data.try_lock()?.combat.target = Target::default();
                                }
                            }

                            npc_clear_move_path(world, entity)?;
                            Ok(())
                        }
                        _ => Ok(()),
                    },
                    None => Ok(()),
                }
            } else {
                Ok(())
            }
        })()?;

        let (clear_move_path, entity_pos) = {
            let mut n_data = n_data.try_lock()?;
            let mut clear_move_path = false;

            if n_data.combat.target.target_entity.is_some() {
                if (base.target_auto_switch
                    && n_data.combat.target.target_timer < *storage.gettick.borrow())
                    || (base.target_range_dropout
                        && n_data
                            .movement
                            .pos
                            .checkdistance(n_data.combat.target.target_pos)
                            > base.sight)
                {
                    n_data.combat.target = Target::default();
                    clear_move_path = true;
                } else {
                    return Ok(());
                }
            }

            (clear_move_path, n_data.movement.pos)
        };

        if clear_move_path {
            npc_clear_move_path(world, entity)?;
        }

        if !base.is_agressive() {
            return Ok(());
        }

        let map_range = get_maps_in_range(storage, &entity_pos, base.sight);
        let valid_map_data = map_range
            .iter()
            .filter_map(|map_pos| map_pos.get())
            .filter_map(|i| storage.maps.get(&i));

        for map_data_ref in valid_map_data {
            let map_data = map_data_ref.borrow();

            for x in map_data.players.iter() {
                if npc_targeting(world, storage, entity, base, *x)? {
                    return Ok(());
                }
            }

            if base.has_enemies {
                for x in map_data.npcs.iter() {
                    if *x == entity {
                        continue;
                    }

                    if npc_targeting(world, storage, entity, base, *x)? {
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn try_target_entity(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let (target, pos, npc_index) = {
            let n_data = n_data.try_lock()?;

            (n_data.combat.target, n_data.movement.pos, n_data.index)
        };

        let new_target = target_entity != entity;

        let cantarget = if let Some(old_target_entity) = target.target_entity {
            let mut rng = rng();

            match world.get_opt_entity(old_target_entity) {
                Some(e_data) => match e_data {
                    Entity::Npc(n2_data) => {
                        if rng.random_range(0..2) == 1 && new_target {
                            true
                        } else {
                            let n2_data = n2_data.try_lock()?;

                            !can_target(pos, n2_data.movement.pos, n2_data.combat.death_type, 1)
                        }
                    }
                    Entity::Player(p_data) => {
                        if rng.random_range(0..2) == 1 && new_target {
                            true
                        } else {
                            let p_data = p_data.try_lock()?;

                            !can_target(pos, p_data.movement.pos, p_data.combat.death_type, 1)
                        }
                    }
                    _ => true,
                },
                None => true,
            }
        } else {
            true
        };

        let npc_base = storage.bases.npcs.get(npc_index as usize);

        if let Some(base) = npc_base
            && cantarget
        {
            if let Some(e_data) = world.get_opt_entity(target_entity) {
                match e_data {
                    Entity::Npc(n2_data) => {
                        let (can_proceed, target_pos) = {
                            let n2_data = n2_data.try_lock()?;

                            (
                                can_target(
                                    pos,
                                    n2_data.movement.pos,
                                    n2_data.combat.death_type,
                                    base.sight,
                                ),
                                n2_data.movement.pos,
                            )
                        };

                        if can_proceed {
                            let mut n_data = n_data.try_lock()?;

                            n_data.combat.target.target_pos = target_pos;
                            n_data.combat.target.target_entity = Some(target_entity);
                            n_data.combat.target.target_timer = *storage.gettick.borrow()
                                + Duration::try_milliseconds(base.target_auto_switch_chance)
                                    .unwrap_or_default();
                        }
                    }
                    Entity::Player(p_data) => {
                        let (can_proceed, target_pos) = {
                            let p_data = p_data.try_lock()?;

                            (
                                can_target(
                                    pos,
                                    p_data.movement.pos,
                                    p_data.combat.death_type,
                                    base.sight,
                                ),
                                p_data.movement.pos,
                            )
                        };

                        if can_proceed {
                            let mut n_data = n_data.try_lock()?;

                            n_data.combat.target.target_pos = target_pos;
                            n_data.combat.target.target_entity = Some(target_entity);
                            n_data.combat.target.target_timer = *storage.gettick.borrow()
                                + Duration::try_milliseconds(base.target_auto_switch_chance)
                                    .unwrap_or_default();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

pub fn update_target_pos(world: &mut World, entity: GlobalKey) -> Result<Target> {
    if !world.entities.contains_key(entity) {
        return Ok(Target::default());
    }

    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        let mut target = n_data.combat.target;

        if let Some(target_entity) = n_data.combat.target.target_entity {
            if target_entity != entity {
                if let Some(e_data) = world.get_opt_entity(target_entity) {
                    match e_data {
                        Entity::Player(p_data) => {
                            let p_data = p_data.try_lock()?;

                            let target_pos = p_data.movement.pos;
                            let deathtype = p_data.combat.death_type;

                            if check_surrounding(n_data.movement.pos.map, target_pos.map, true)
                                == MapPos::None
                                || !deathtype.is_alive()
                            {
                                target = Target::default();
                            } else {
                                target.target_pos = target_pos;
                            }
                        }
                        Entity::Npc(n2_data) => {
                            let n2_data = n2_data.try_lock()?;

                            let target_pos = n2_data.movement.pos;
                            let deathtype = n2_data.combat.death_type;

                            if check_surrounding(n_data.movement.pos.map, target_pos.map, true)
                                == MapPos::None
                                || !deathtype.is_alive()
                            {
                                target = Target::default();
                            } else {
                                target.target_pos = target_pos;
                            }
                        }
                        _ => {}
                    }

                    n_data.combat.target = target;
                }
            }
        }

        Ok(target)
    } else {
        Ok(Target::default())
    }
}

pub fn npc_targeting(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    base: &NpcData,
    target_entity: GlobalKey,
) -> Result<bool> {
    if target_entity == entity {
        return Ok(false);
    }

    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let entity_pos = { n_data.try_lock()?.movement.pos };

        let (pos, _) = match world.get_opt_entity(target_entity) {
            Some(e_result) => {
                match e_result {
                    Entity::Player(p_data) => {
                        let p_data = p_data.try_lock()?;

                        if p_data.combat.death_type.is_alive() {
                            let check =
                                check_surrounding(entity_pos.map, p_data.movement.pos.map, true);
                            let pos = p_data.movement.pos.map_offset(check.into());
                            let dir = p_data.movement.dir;
                            (pos, dir)
                        } else {
                            return Ok(false);
                        }
                    }
                    Entity::Npc(n2_data) => {
                        let n2_data = n2_data.try_lock()?;

                        //let newbase = &storage.bases.npcs[world.get_or_err::<NpcIndex>(&i)?.0 as usize];
                        let mut is_enemy = false;

                        if base.has_enemies {
                            is_enemy = base.enemies.iter().any(|&x| n2_data.index == x);
                        }

                        if n2_data.combat.death_type.is_alive() && is_enemy {
                            let check =
                                check_surrounding(entity_pos.map, n2_data.movement.pos.map, true);
                            let pos = n2_data.movement.pos.map_offset(check.into());
                            let dir = n2_data.movement.dir;
                            (pos, dir)
                        } else {
                            return Ok(false);
                        }
                    }
                    _ => return Ok(false),
                }
            }
            None => return Ok(false),
        };

        let distance = entity_pos.checkdistance(pos);
        if distance > base.sight {
            return Ok(false);
        }

        let mut n_data = n_data.try_lock()?;

        n_data.combat.target.target_pos = pos;
        n_data.combat.target.target_entity = Some(target_entity);
        n_data.combat.target.target_timer = *storage.gettick.borrow()
            + Duration::try_milliseconds(base.target_auto_switch_chance).unwrap_or_default();
        n_data.combat.attack_timer.0 = *storage.gettick.borrow()
            + Duration::try_milliseconds(base.attack_wait).unwrap_or_default();

        Ok(true)
    } else {
        Ok(false)
    }
}
