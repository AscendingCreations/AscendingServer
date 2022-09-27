use crate::{containers::Storage, gametypes::*, maps::*, npcs::*};
use rand::{thread_rng, Rng};

pub fn entity_cast_check(_world: &Storage, caster: &Entity, target: &Entity, range: i32) -> bool {
    let check = check_surrounding(caster.pos.map, target.pos.map, true);
    let pos = target.pos.map_offset(check.into());

    range >= caster.pos.checkdistance(pos) && target.life.is_alive()
}

pub fn try_cast(
    world: &Storage,
    caster: &Entity,
    base: &NpcData,
    target: EntityType,
    range: i32,
    casttype: NpcCastType,
) -> bool {
    match target {
        EntityType::Player(i, accid) => {
            if let Some(target) = world.players.borrow().get(i as usize) {
                if (base.can_attack_player || matches!(caster.mode, NpcMode::Pet | NpcMode::Summon))
                    && target.borrow().accid == accid
                {
                    return entity_cast_check(world, caster, &target.borrow().e, range);
                }
            }
        }
        EntityType::Npc(i) => {
            if let Some(target) = world.npcs.borrow().get(i as usize) {
                let target = target.borrow();

                if base.has_enemies
                    && casttype == NpcCastType::Enemy
                    && base.enemies.iter().any(|e| *e == target.num)
                {
                    return entity_cast_check(world, caster, &target.e, range);
                }
            }
        }
        EntityType::Map(_) | EntityType::None => {}
    }

    false
}

pub fn npc_cast(world: &Storage, npc: &mut Npc, base: &NpcData) -> Option<EntityType> {
    match base.behaviour {
        AIBehavior::Agressive
        | AIBehavior::AgressiveHealer
        | AIBehavior::ReactiveHealer
        | AIBehavior::HelpReactive
        | AIBehavior::Reactive => {
            if try_cast(
                world,
                &npc.e,
                base,
                npc.e.targettype,
                base.range,
                NpcCastType::Enemy,
            ) {
                return Some(npc.e.targettype);
            }

            None
        }
        AIBehavior::Healer | AIBehavior::Friendly => None,
    }
}

pub fn npc_combat(world: &Storage, npc: &mut Npc, base: &NpcData) {
    if let Some(entitytype) = npc_cast(world, npc, base) {
        match entitytype {
            EntityType::Player(i, _accid) => {
                if let Some(target) = world.players.borrow().get(i as usize) {
                    let damage = npc_combat_damage(&target.borrow().e, npc, base);
                    target.borrow_mut().damage_player(damage)

                    //TODO Send Attack And Damage packets here.
                }
            }
            EntityType::Npc(i) => {
                if let Some(target) = world.npcs.borrow().get(i as usize) {
                    let damage = npc_combat_damage(&target.borrow().e, npc, base);
                    target.borrow_mut().damage_npc(damage)

                    //TODO Send Attack And Damage packets here.
                }
            }
            EntityType::Map(_) | EntityType::None => {}
        }
    }
}

pub fn npc_combat_damage(entity: &Entity, npc: &mut Npc, base: &NpcData) -> i32 {
    let def = if entity.etype.is_player() {
        entity.pdefense + entity.level.saturating_div(5) as u32
    } else {
        entity.pdefense
    };

    let offset = if entity.etype.is_player() { 4 } else { 2 };

    let mut damage = npc.e.pdamage.saturating_sub(def / offset);
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
