use crate::{containers::Storage, gametypes::*, maps::*, npcs::*};

pub fn entity_cast_check(world: &Storage, caster: &Entity, target: &Entity, range: i32) -> bool {
    let check = check_surrounding(caster.pos.map, target.pos.map, true);
    let pos = target.pos.map_offset(check.into());

    if range >= caster.pos.checkdistance(pos) && target.life.is_alive() {
        true
    } else {
        false
    }
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

pub fn npc_cast(
    world: &Storage,
    npc: &mut Npc,
    base: &NpcData,
) -> Option<(Option<Position>, EntityType)> {
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
                return Some((None, npc.e.targettype));
            }

            None
        }
        AIBehavior::Healer | AIBehavior::Friendly => None,
    }
}

pub fn npc_combat(world: &Storage, npc: &mut Npc, base: &NpcData) {
    let cast = npc_cast(world, npc, base);

    if let Some((pos, entitytype)) = cast {
        let _startpos = if entitytype != EntityType::None {
            if let Some(newpos) = entitytype.get_pos(world) {
                newpos
            } else {
                npc.e.pos
            }
        } else if let Some(spos) = pos {
            spos
        } else {
            npc.e.pos
        };

        /*todo handle Damage and attack here. */
    }
}
