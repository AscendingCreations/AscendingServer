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

                if casttype == NpcCastType::Enemy && base.enemies.iter().any(|e| *e == target.num) {
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
            EntityType::Player(i, accid) => {
                if let Some(mut target) = world.players.borrow().get(i as usize) {
                    let def = target.borrow().e.pdefense;
                    let mut damage = npc.e.pdamage.saturating_sub(def / 4);

                    //protect from accidental heals due to u32 to i32 conversion.
                    if damage >= i32::MAX as u32 {
                        damage = (i32::MAX - 1) as u32;
                    }

                    //lets randomize to see if we do want to deal 1 damage if Defense is to high.
                    if damage == 0 {
                        let mut rng = thread_rng();
                        damage = rng.gen_range(0..=1);
                    }

                    target.borrow_mut().damage_player(damage as i32)

                    //TODO Send Attack And Damage packets here.
                }
            }
            EntityType::Npc(i) => {
                if let Some(target) = world.npcs.borrow().get(i as usize) {
                    let def = target.borrow().e.pdefense;
                    let mut damage = npc.e.pdamage.saturating_sub(def / 2);

                    //protect from accidental heals due to u32 to i32 conversion.
                    if damage >= i32::MAX as u32 {
                        damage = (i32::MAX - 1) as u32;
                    }

                    //lets randomize to see if we do want to deal 1 damage if Defense is to high.
                    if damage == 0 {
                        let mut rng = thread_rng();
                        damage = rng.gen_range(0..=1);
                    }

                    target.borrow_mut().damage_npc(damage as i32)

                    //TODO Send Attack And Damage packets here.
                }
            }
            EntityType::Map(_) | EntityType::None => {}
        }
    }
}
