use std::borrow::Borrow;

use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, players::*, tasks::*};
use hecs::World;
use rand::{thread_rng, Rng};

#[inline(always)]
pub fn damage_npc(world: &mut World, entity: &crate::Entity, damage: i32) {
    world
        .get::<&mut Vitals>(entity.0)
        .expect("Could not find Position")
        .vital[VitalTypes::Hp as usize] =
        world.get_or_panic::<Vitals>(entity).vital[VitalTypes::Hp as usize].saturating_sub(damage);
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
    caster: &Entity,
    base: &NpcData,
    target: EntityType,
    range: i32,
    casttype: NpcCastType,
) -> bool {
    if !world.contains(caster.0) {
        return false;
    }

    let caster_pos = world.get_or_default::<Position>(caster);
    let npc_mode = world.get_or_default::<NpcMode>(caster);

    match target {
        EntityType::Player(i, _accid) => {
            if world.contains(i.0)
                && (base.can_attack_player || matches!(npc_mode, NpcMode::Pet | NpcMode::Summon))
            {
                let target_pos = world.get_or_default::<Position>(&i);
                let life = world.get_or_default::<DeathType>(&i);
                return entity_cast_check(caster_pos, target_pos, life, range);
            }
        }
        EntityType::Npc(i) => {
            if base.has_enemies
                && casttype == NpcCastType::Enemy
                && base
                    .enemies
                    .iter()
                    .any(|e| *e == world.get_or_panic::<NpcIndex>(&i).0)
            {
                let target_pos = world.get_or_default::<Position>(&i);
                let life = world.get_or_default::<DeathType>(&i);
                return entity_cast_check(caster_pos, target_pos, life, range);
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
    }

    false
}

pub fn npc_cast(world: &mut World, npc: &Entity, base: &NpcData) -> Option<EntityType> {
    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            if let Ok(targettype) = world.get::<&Target>(npc.0).map(|t| t.targettype) {
                if try_cast(world, npc, base, targettype, base.range, NpcCastType::Enemy) {
                    return Some(targettype);
                }
            }

            None
        }
        AIBehavior::Healer | AIBehavior::Friendly => None,
    }
}

pub fn npc_combat(world: &mut World, storage: &Storage, entity: &Entity, base: &NpcData) {
    if let Some(entitytype) = npc_cast(world, entity, base) {
        match entitytype {
            EntityType::Player(i, _accid) => {
                if world.contains(i.0) {
                    let damage = npc_combat_damage(world, entity, &i, base);
                    damage_player(world, &i, damage);

                    let _ = DataTaskToken::NpcAttack(world.get_or_default::<Position>(entity).map)
                        .add_task(storage, entity);
                    let vitals = world.get_or_panic::<Vitals>(&i);
                    if vitals.vital[0] > 0 {
                        let _ = DataTaskToken::PlayerVitals(
                            world.get_or_default::<Position>(entity).map,
                        )
                        .add_task(storage, {
                            let vitals = world.get_or_panic::<Vitals>(&i);

                            &VitalsPacket::new(i, vitals.vital, vitals.vitalmax)
                        });
                    } else {
                        remove_all_npc_target(world, &i);
                        kill_player(world, storage, &i);
                    }
                    //TODO Send Attack Msg/Damage
                }
            }
            EntityType::Npc(i) => {
                if world.contains(i.0) {
                    let damage = npc_combat_damage(world, entity, &i, base);
                    damage_npc(world, &i, damage);

                    let _ = DataTaskToken::NpcAttack(world.get_or_default::<Position>(entity).map)
                        .add_task(storage, entity);
                    let _ = DataTaskToken::NpcVitals(world.get_or_default::<Position>(entity).map)
                        .add_task(storage, {
                            let vitals = world.get_or_panic::<Vitals>(&i);

                            &VitalsPacket::new(i, vitals.vital, vitals.vitalmax)
                        });
                }
            }
            EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
        }
    }
}

pub fn npc_combat_damage(
    world: &mut World,
    entity: &Entity,
    enemy_entity: &Entity,
    base: &NpcData,
) -> i32 {
    let data = world.entity(entity.0).expect("Could not get Entity");
    let edata = world.entity(enemy_entity.0).expect("Could not get Entity");

    let def = if edata.get_or_panic::<WorldEntityType>() == WorldEntityType::Player {
        edata.get_or_panic::<Physical>().defense
            + edata.get_or_panic::<Level>().0.saturating_div(5) as u32
    } else {
        edata.get_or_panic::<Physical>().defense
    };

    let offset = if edata.get_or_panic::<WorldEntityType>() == WorldEntityType::Player {
        4
    } else {
        2
    };

    let mut damage = data
        .get_or_panic::<Physical>()
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

    damage as i32
}

pub fn kill_npc(world: &mut World, storage: &Storage, entity: &Entity) {
    let npc_index = world.get_or_panic::<NpcIndex>(entity).0;
    let npc_pos = world.get_or_panic::<Position>(entity);
    let npcbase = storage.bases.npcs[npc_index as usize].borrow();

    let mut rng = thread_rng();

    let mut count = 0;
    for index in 0..10 {
        if npcbase.drops[index].0 > 0 && rng.gen_range(1..=npcbase.drops[index].2) == 1 {
            if !try_drop_item(
                world,
                storage,
                DropItem {
                    index: npcbase.drops[index].0,
                    amount: npcbase.drops[index].1 as u16,
                    pos: npc_pos,
                },
                None,
                None,
                None,
            ) {
                break;
            }

            count += 1;
            if count >= npcbase.drops_max {
                break;
            }
        }
    }

    *world
        .get::<&mut DeathType>(entity.0)
        .expect("Could not find DeathType") = DeathType::Dead;
}
