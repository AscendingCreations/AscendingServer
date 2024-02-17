use crate::{containers::Storage, gametypes::*, maps::*, npcs::*, tasks::*, players::*,};
use rand::{thread_rng, Rng};

pub fn entity_cast_check(world: &mut hecs::World, caster: &Entity, target: &Entity, range: i32) -> bool {
    let cdata = world.entity(caster.0).expect("Could not get Entity");
    let tdata = world.entity(target.0).expect("Could not get Entity");

    let check = check_surrounding(cdata.get::<&Position>().expect("Could not find Position").map, 
    tdata.get::<&Position>().expect("Could not find Position").map, true);
    let pos = tdata.get::<&Position>().expect("Could not find Position").map_offset(check.into());

    range >= cdata.get::<&Position>().expect("Could not find Position").checkdistance(pos) && 
        tdata.get::<&DeathType>().expect("Could not find DeathType").is_alive()
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
    let data = world.entity(caster.0).expect("Could not get Entity");
    match target {
        EntityType::Player(i, accid) => {
            if let Ok(target) = world.get::<&Account>(i.0) {
                if (base.can_attack_player || 
                    matches!(*data.get::<&NpcMode>().expect("Could not find NpcMode"), NpcMode::Pet | NpcMode::Summon))
                    && target.id == accid
                {
                    return entity_cast_check(world, caster, &i, range);
                }
            }
        }
        EntityType::Npc(i) => {
            if let edata = world.entity(i.0).expect("Could not get Entity") {
                if base.has_enemies
                    && casttype == NpcCastType::Enemy
                    && base.enemies.iter().any(|e| *e == edata.get::<&NpcIndex>().expect("Could not find NpcMode").0)
                {
                    return entity_cast_check(world, caster, &i, range);
                }
            }
        }
        EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
    }

    false
}

pub fn npc_cast(world: &mut hecs::World, storage: &Storage, npc: &Entity, base: &NpcData) -> Option<EntityType> {
    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            let data = world.entity(npc.0).expect("Could not get Entity");
            let targettype = data.get::<&Target>().expect("Could not find Target");
            if try_cast(
                world,
                storage,
                npc,
                base,
                targettype.targettype,
                base.range,
                NpcCastType::Enemy,
            ) {
                return Some(targettype.targettype);
            }

            None
        }
        AIBehavior::Healer | AIBehavior::Friendly => None,
    }
}

pub fn npc_combat(world: &mut hecs::World, storage: &Storage, entity: &Entity, base: &NpcData) {
    let data = world.entity(entity.0).expect("Could not get Entity");
    if let Some(entitytype) = npc_cast(world, storage, entity, base) {
        match entitytype {
            EntityType::Player(i, _accid) => {
                if let Ok(edata) = world.entity(i.0) {
                    let damage = npc_combat_damage(world, entity, &i, base);
                    damage_player(world, &i, damage);

                    let _ = DataTaskToken::NpcAttack(data.get::<&Position>().expect("Could not find Position").map)
                        .add_task(world, storage, &(*entity));
                    let _ = DataTaskToken::PlayerVitals(data.get::<&Position>().expect("Could not find Position").map).add_task(
                        world,
                        storage,
                        &VitalsPacket::new(
                            i,
                            edata.get::<&Vitals>().expect("Could not find Vitals").vital,
                            edata.get::<&Vitals>().expect("Could not find Vitals").vitalmax,
                        ),
                    );
                    //TODO Send Attack Msg/Damage
                }
            }
            EntityType::Npc(i) => {
                if let Ok(edata) = world.entity(i.0) {
                    let damage = npc_combat_damage(world, entity, &i, base);
                    damage_npc(world, &i, damage);

                    let _ = DataTaskToken::NpcAttack(data.get::<&Position>().expect("Could not find Position").map)
                        .add_task(world, storage, &(*entity));
                    let _ = DataTaskToken::NpcVitals(data.get::<&Position>().expect("Could not find Position").map).add_task(
                        world,
                        storage,
                        &VitalsPacket::new(
                            i,
                            edata.get::<&Vitals>().expect("Could not find Vitals").vital,
                            edata.get::<&Vitals>().expect("Could not find Vitals").vitalmax,
                        ),
                    );
                }
            }
            EntityType::Map(_) | EntityType::None | EntityType::MapItem(_) => {}
        }
    }
}

pub fn npc_combat_damage(world: &mut hecs::World, entity: &Entity, enemy_entity: &Entity, base: &NpcData) -> i32 {
    let data = world.entity(entity.0).expect("Could not get Entity");
    let edata = world.entity(enemy_entity.0).expect("Could not get Entity");

    let def = if *edata.get::<&WorldEntityType>().expect("Could not find WorldEntityType") == WorldEntityType::Player {
        edata.get::<&Physical>().expect("Could not find Physical").defense + 
            edata.get::<&Level>().expect("Could not find Physical").0.saturating_div(5) as u32
    } else {
        edata.get::<&Physical>().expect("Could not find Physical").defense
    };

    let offset = 
        if *edata.get::<&WorldEntityType>().expect("Could not find WorldEntityType") == WorldEntityType::Player
            { 4 } else { 2 };

    let mut damage = data.get::<&Physical>().expect("Could not find Physical").damage.saturating_sub(def / offset);
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
