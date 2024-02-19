use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, players::*, tasks::*};
use rand::{thread_rng, Rng};

pub fn entity_cast_check(
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
    world: &mut hecs::World,
    storage: &Storage,
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
        EntityType::Player(i, accid) => {
            if let Ok(target) = world.get::<&Account>(i.0).map(|acc| acc.id) {
                if (base.can_attack_player || matches!(npc_mode, NpcMode::Pet | NpcMode::Summon))
                    && target == accid
                {
                    let target_pos = world.get_or_default::<Position>(&i);
                    let life = world.get_or_default::<DeathType>(&i);
                    return entity_cast_check(caster_pos, target_pos, life, range);
                }
            }
        }
        EntityType::Npc(i) => {
            if base.has_enemies
                && casttype == NpcCastType::Enemy
                && base.enemies.iter().any(|e| {
                    *e == world
                        .get::<&NpcIndex>(i.0)
                        .expect("Could not find NpcIndex")
                        .0
                })
            {
                let target_pos = world.get_or_default::<Position>(&i);
                let life = world.get_or_default::<DeathType>( &i);
                return entity_cast_check(caster_pos, target_pos, life, range);
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
    }

    false
}

pub fn npc_cast(
    world: &mut hecs::World,
    storage: &Storage,
    npc: &Entity,
    base: &NpcData,
) -> Option<EntityType> {
    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            if let Ok(targettype) = world.get::<&Target>(npc.0).map(|t| t.targettype) {
                if try_cast(
                    world,
                    storage,
                    npc,
                    base,
                    targettype,
                    base.range,
                    NpcCastType::Enemy,
                ) {
                    return Some(targettype);
                }
            }

            None
        }
        AIBehavior::Healer | AIBehavior::Friendly => None,
    }
}

pub fn npc_combat(world: &mut hecs::World, storage: &Storage, entity: &Entity, base: &NpcData) {
    //let data = world.entity(entity.0).expect("Could not get Entity");
    if let Some(entitytype) = npc_cast(world, storage, entity, base) {
        match entitytype {
            EntityType::Player(i, _accid) => {
                if world.contains(i.0) {
                    let damage = npc_combat_damage(world, entity, &i, base);
                    damage_player(world, &i, damage);

                    let _ = DataTaskToken::NpcAttack(world.get_or_default::<Position>(entity).map)
                        .add_task(world, storage, &(*entity));
                    let _ =
                        DataTaskToken::PlayerVitals(world.get_or_default::<Position>(entity).map)
                            .add_task(world, storage, {
                                let vitals =
                                    world.get::<&Vitals>(i.0).expect("Could not find Vitals");

                                &VitalsPacket::new(i, vitals.vital, vitals.vitalmax)
                            });
                    //TODO Send Attack Msg/Damage
                }
            }
            EntityType::Npc(i) => {
                if world.contains(i.0) {
                    let damage = npc_combat_damage(world, entity, &i, base);
                    damage_npc(world, &i, damage);

                    let _ = DataTaskToken::NpcAttack(world.get_or_default::<Position>(entity).map)
                        .add_task(world, storage, &(*entity));
                    let _ = DataTaskToken::NpcVitals(world.get_or_default::<Position>(entity).map)
                        .add_task(world, storage, {
                            let vitals = world.get::<&Vitals>(i.0).expect("Could not find Vitals");

                            &VitalsPacket::new(i, vitals.vital, vitals.vitalmax)
                        });
                }
            }
            EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
        }
    }
}

pub fn npc_combat_damage(
    world: &mut hecs::World,
    entity: &Entity,
    enemy_entity: &Entity,
    base: &NpcData,
) -> i32 {
    let data = world.entity(entity.0).expect("Could not get Entity");
    let edata = world.entity(enemy_entity.0).expect("Could not get Entity");

    let def = if *edata
        .get::<&WorldEntityType>()
        .expect("Could not find WorldEntityType")
        == WorldEntityType::Player
    {
        edata
            .get::<&Physical>()
            .expect("Could not find Physical")
            .defense
            + edata
                .get::<&Level>()
                .expect("Could not find Physical")
                .0
                .saturating_div(5) as u32
    } else {
        edata
            .get::<&Physical>()
            .expect("Could not find Physical")
            .defense
    };

    let offset = if *edata
        .get::<&WorldEntityType>()
        .expect("Could not find WorldEntityType")
        == WorldEntityType::Player
    {
        4
    } else {
        2
    };

    let mut damage = data
        .get::<&Physical>()
        .expect("Could not find Physical")
        .damage
        .saturating_sub(def / offset);
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
