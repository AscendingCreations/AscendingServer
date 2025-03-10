use std::borrow::Borrow;

use crate::{
    containers::{GlobalKey, Storage, World},
    gametypes::*,
    maps::*,
    npcs::*,
    players::*,
    tasks::*,
};
use rand::{Rng, rng};

#[inline(always)]
pub fn damage_npc(world: &mut World, entity: GlobalKey, damage: i32) -> Result<()> {
    world.get::<&mut Vitals>(entity.0)?.vital[VitalTypes::Hp as usize] =
        world.get_or_err::<Vitals>(entity)?.vital[VitalTypes::Hp as usize].saturating_sub(damage);
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
    target: EntityType,
    range: i32,
    casttype: NpcCastType,
) -> Result<bool> {
    if !world.contains(caster.0) {
        return Ok(false);
    }

    let caster_pos = world.get_or_default::<Position>(caster);
    let npc_mode = world.get_or_default::<NpcMode>(caster);

    match target {
        EntityType::Player(i, _accid) => {
            if world.contains(i.0)
                && (base.can_attack_player || matches!(npc_mode, NpcMode::Pet | NpcMode::Summon))
                && !world.get_or_err::<IsUsingType>(&i)?.inuse()
            {
                let target_pos = world.get_or_default::<Position>(&i);
                let life = world.get_or_default::<DeathType>(&i);

                if let Some(dir) = caster_pos.checkdirection(target_pos) {
                    if is_dir_blocked(storage, caster_pos, dir as u8) {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }

                return Ok(entity_cast_check(caster_pos, target_pos, life, range));
            }
        }
        EntityType::Npc(i) => {
            if base.has_enemies
                && casttype == NpcCastType::Enemy
                && base
                    .enemies
                    .iter()
                    .any(|e| *e == world.get_or_default::<NpcIndex>(&i).0)
            {
                let target_pos = world.get_or_default::<Position>(&i);
                let life = world.get_or_default::<DeathType>(&i);

                if let Some(dir) = caster_pos.checkdirection(target_pos) {
                    if is_dir_blocked(storage, caster_pos, dir as u8) {
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

pub fn npc_cast(
    world: &mut World,
    storage: &Storage,
    npc: GlobalKey,
    base: &NpcData,
) -> Result<EntityType> {
    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            if let Ok(targettype) = world.get::<&Target>(npc.0).map(|t| t.target_type) {
                if try_cast(
                    world,
                    storage,
                    npc,
                    base,
                    targettype,
                    base.range,
                    NpcCastType::Enemy,
                )? {
                    return Ok(targettype);
                }
            }

            Ok(EntityType::None)
        }
        AIBehavior::Healer | AIBehavior::Friendly => Ok(EntityType::None),
    }
}

pub fn can_attack_npc(world: &mut World, storage: &Storage, npc: GlobalKey) -> Result<bool> {
    let npc_index = world.get_or_err::<NpcIndex>(npc)?.0;
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

pub fn npc_combat(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    base: &NpcData,
) -> Result<()> {
    match npc_cast(world, storage, entity, base)? {
        EntityType::Player(i, _accid) => {
            if world.contains(i.0) {
                let damage = npc_combat_damage(world, storage, entity, &i, base)?;
                damage_player(world, &i, damage)?;
                DataTaskToken::Damage(world.get_or_default::<Position>(entity).map).add_task(
                    storage,
                    damage_packet(
                        *entity,
                        damage as u16,
                        world.get_or_default::<Position>(&i),
                        true,
                    )?,
                )?;
                DataTaskToken::Attack(world.get_or_default::<Position>(entity).map)
                    .add_task(storage, attack_packet(*entity)?)?;
                let vitals = world.get_or_err::<Vitals>(&i)?;
                if vitals.vital[0] > 0 {
                    DataTaskToken::Vitals(world.get_or_default::<Position>(entity).map).add_task(
                        storage,
                        {
                            let vitals = world.get_or_err::<Vitals>(&i)?;

                            vitals_packet(i, vitals.vital, vitals.vitalmax)?
                        },
                    )?;
                } else {
                    remove_all_npc_target(world, &i)?;
                    kill_player(world, storage, &i)?;
                }
            }
        }
        EntityType::Npc(i) => {
            if world.contains(i.0) {
                let damage = npc_combat_damage(world, storage, entity, &i, base)?;
                damage_npc(world, &i, damage)?;

                DataTaskToken::Damage(world.get_or_default::<Position>(entity).map).add_task(
                    storage,
                    damage_packet(
                        *entity,
                        damage as u16,
                        world.get_or_default::<Position>(&i),
                        true,
                    )?,
                )?;
                DataTaskToken::Attack(world.get_or_default::<Position>(entity).map)
                    .add_task(storage, attack_packet(*entity)?)?;

                let vitals = world.get_or_err::<Vitals>(&i)?;
                if vitals.vital[0] > 0 {
                    DataTaskToken::Vitals(world.get_or_default::<Position>(entity).map)
                        .add_task(storage, {
                            vitals_packet(i, vitals.vital, vitals.vitalmax)?
                        })?;
                    try_target_entity(world, storage, &i, EntityType::Npc(*entity))?;
                } else {
                    kill_npc(world, storage, &i)?;
                }
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
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
    let def = if world.get_or_err::<EntityKind>(enemy_entity)? == EntityKind::Player {
        world.get_or_err::<Physical>(enemy_entity)?.defense
            + player_get_armor_defense(world, storage, entity)?.0 as u32
            + world.get_or_err::<Level>(enemy_entity)?.0.saturating_div(5) as u32
    } else {
        world.get_or_err::<Physical>(enemy_entity)?.defense
    };

    let data = world.entity(entity.0)?;
    let edata = world.entity(enemy_entity.0)?;

    let offset = if edata.get_or_err::<EntityKind>()? == EntityKind::Player {
        4
    } else {
        2
    };

    let mut damage = data
        .get_or_err::<Physical>()?
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
}

pub fn kill_npc(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let npc_index = world.get_or_err::<NpcIndex>(entity)?.0;
    let npc_pos = world.get_or_err::<Position>(entity)?;
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

    *world.get::<&mut DeathType>(entity.0)? = DeathType::Dead;
    Ok(())
}
