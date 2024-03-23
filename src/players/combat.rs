use crate::{
    containers::Storage,
    gametypes::*,
    maps::check_surrounding,
    npcs::{damage_npc, kill_npc, try_target_entity},
    players::*,
    tasks::{init_data_lists, DataTaskToken, VitalsPacket},
};
use hecs::World;
use rand::*;
use std::cmp;

#[inline]
pub fn damage_player(world: &mut World, entity: &crate::Entity, damage: i32) {
    let mut query = world
        .query_one::<&mut Vitals>(entity.0)
        .expect("damage_player could not find query");

    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize] =
            player_vital.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }
}

pub fn get_damage_percentage(damage: u32, hp: (u32, u32)) -> f64 {
    let curhp = cmp::min(hp.0, hp.1);
    let abs_damage = cmp::min(damage, curhp) as f64;
    abs_damage / curhp as f64
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

pub fn try_player_cast(world: &mut World, caster: &Entity, target: &Entity) -> bool {
    if !world.contains(caster.0) || !world.contains(target.0) {
        return false;
    }

    let caster_pos = world.get_or_default::<Position>(caster);
    let target_pos = world.get_or_default::<Position>(target);
    let life = world.get_or_default::<DeathType>(target);

    entity_cast_check(caster_pos, target_pos, life, 1)
}

pub fn player_combat(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    target_entity: &Entity,
) {
    if try_player_cast(world, entity, target_entity) {
        let world_entity_type = world.get_or_default::<WorldEntityType>(target_entity);
        match world_entity_type {
            WorldEntityType::Player => {
                let damage = player_combat_damage(world, entity, target_entity);
                damage_player(world, target_entity, damage);

                let _ = DataTaskToken::PlayerAttack(world.get_or_default::<Position>(entity).map)
                    .add_task(storage, entity);

                let vitals = world.get_or_panic::<Vitals>(target_entity);
                if vitals.vital[0] > 0 {
                    let _ =
                        DataTaskToken::PlayerVitals(world.get_or_default::<Position>(entity).map)
                            .add_task(storage, {
                                &VitalsPacket::new(*target_entity, vitals.vital, vitals.vitalmax)
                            });
                } else {
                    kill_player(world, storage, target_entity);
                }
            }
            WorldEntityType::Npc => {
                let damage = player_combat_damage(world, entity, target_entity);
                damage_npc(world, target_entity, damage);

                let _ = DataTaskToken::PlayerAttack(world.get_or_default::<Position>(entity).map)
                    .add_task(storage, entity);

                let vitals = world.get_or_panic::<Vitals>(target_entity);
                if vitals.vital[0] > 0 {
                    let _ = DataTaskToken::NpcVitals(world.get_or_default::<Position>(entity).map)
                        .add_task(storage, {
                            &VitalsPacket::new(*target_entity, vitals.vital, vitals.vitalmax)
                        });

                    let acc_id = world.cloned_get_or_default::<Account>(entity).id;
                    try_target_entity(
                        world,
                        storage,
                        target_entity,
                        EntityType::Player(*entity, acc_id),
                    )
                } else {
                    kill_npc(world, storage, target_entity);
                }
            }
            _ => {}
        }
    }
}

pub fn player_combat_damage(world: &mut World, entity: &Entity, target_entity: &Entity) -> i32 {
    let data = world.entity(entity.0).expect("Could not get Entity");
    let edata = world.entity(target_entity.0).expect("Could not get Entity");

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

    damage as i32
}

pub fn kill_player(world: &mut World, storage: &Storage, entity: &Entity) {
    {
        if let Ok(mut vitals) = world.get::<&mut Vitals>(entity.0) {
            vitals.vital = vitals.vitalmax;
        }
        world
            .get::<&mut PlayerTarget>(entity.0)
            .expect("Could not find PlayerTarget")
            .0 = None;
    }
    let _ = DataTaskToken::PlayerVitals(world.get_or_default::<Position>(entity).map).add_task(
        storage,
        {
            let vitals = world.get_or_panic::<Vitals>(entity);

            &VitalsPacket::new(*entity, vitals.vital, vitals.vitalmax)
        },
    );
    let spawn = world.get_or_panic::<Spawn>(entity);
    player_warp(world, storage, entity, &spawn.pos, false);
    init_data_lists(world, storage, entity, None);
}
