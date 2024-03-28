use std::collections::VecDeque;

use hecs::World;

use crate::{containers::*, gametypes::*, tasks::*, time_ext::MyInstant};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct NpcIndex(pub u64);

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcTimer {
    #[derivative(Default(value = "MyInstant::now()"))]
    pub despawntimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub spawntimer: MyInstant,
}

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcAITimer(#[derivative(Default(value = "MyInstant::now()"))] pub MyInstant); //for rebuilding the a* paths

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcDespawns(#[derivative(Default(value = "false"))] pub bool);

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcMoving(#[derivative(Default(value = "false"))] pub bool);

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcRetreating(#[derivative(Default(value = "false"))] pub bool);

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcWalkToSpawn(#[derivative(Default(value = "false"))] pub bool);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
//offset for special things so the npc wont to events based on this spawn time.
pub struct NpcHitBy(#[derivative(Default(value = "Vec::new()"))] pub Vec<(u32, u64, u64)>);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcMoves(#[derivative(Default(value = "VecDeque::new()"))] pub VecDeque<(Position, u8)>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcSpawnedZone(pub Option<usize>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcMovePos(pub Option<Position>);

#[inline(always)]
pub fn is_npc_same(from_entity: &crate::Entity, to_entity: &crate::Entity) -> bool {
    from_entity == to_entity
}

#[inline(always)]
pub fn npc_set_move_path(
    world: &mut World,
    entity: &crate::Entity,
    path: VecDeque<(Position, u8)>,
) {
    world
        .get::<&mut NpcMoves>(entity.0)
        .expect("Could not find NpcMoves")
        .0 = path;
    world
        .get::<&mut NpcMoving>(entity.0)
        .expect("Could not find NpcMoving")
        .0 = true;
}

#[inline(always)]
pub fn npc_clear_move_path(world: &mut World, entity: &crate::Entity) {
    world
        .get::<&mut NpcMoves>(entity.0)
        .expect("Could not find NpcMoves")
        .0
        .clear();
    world
        .get::<&mut NpcMoving>(entity.0)
        .expect("Could not find NpcMoving")
        .0 = false;
}

#[inline(always)]
pub fn set_npc_dir(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    dir: u8,
) -> Result<()> {
    if world.get_or_err::<Dir>(entity)?.0 != dir {
        world
            .get::<&mut Dir>(entity.0)
            .expect("Could not find Dir")
            .0 = dir;

        let _ = DataTaskToken::NpcDir(world.get_or_err::<Position>(entity)?.map)
            .add_task(storage, &DirPacket::new(*entity, dir));
    }

    Ok(())
}

#[inline(always)]
pub fn npc_switch_maps(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    new_pos: Position,
) -> Result<Position> {
    let npc_position = world.get_or_err::<Position>(entity)?;

    let old_position = npc_position;

    if let Some(mapref) = storage.maps.get(&npc_position.map) {
        let mut map = mapref.borrow_mut();
        map.remove_npc(*entity);
        map.remove_entity_from_grid(npc_position);
    } else {
        return Ok(old_position);
    }

    if let Some(mapref) = storage.maps.get(&new_pos.map) {
        let mut map = mapref.borrow_mut();
        map.add_npc(*entity);
        map.add_entity_to_grid(new_pos);
    } else {
        return Ok(old_position);
    }

    *world
        .get::<&mut Position>(entity.0)
        .expect("Could not find Position") = new_pos;

    Ok(old_position)
}

#[inline(always)]
pub fn npc_swap_pos(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    pos: Position,
) -> Result<Position> {
    let oldpos = world.get_or_err::<Position>(entity)?;
    if oldpos != pos {
        *world
            .get::<&mut Position>(entity.0)
            .expect("Could not find Position") = pos;

        let mut map = match storage.maps.get(&oldpos.map) {
            Some(map) => map,
            None => return Ok(oldpos),
        }
        .borrow_mut();
        map.remove_entity_from_grid(oldpos);
        map.add_entity_to_grid(pos);
    }

    Ok(oldpos)
}

pub fn npc_getx(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity)?.x)
}

pub fn npc_gety(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity)?.y)
}

pub fn npc_getmap(world: &mut World, entity: &crate::Entity) -> Result<MapPosition> {
    Ok(world.get_or_err::<Position>(entity)?.map)
}

pub fn npc_gethp(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Vitals>(entity)?.vital[VitalTypes::Hp as usize])
}

pub fn npc_setx(world: &mut World, entity: &crate::Entity, x: i32) {
    world
        .get::<&mut Position>(entity.0)
        .expect("Could not find Position")
        .x = x;
}

pub fn npc_sety(world: &mut World, entity: &crate::Entity, y: i32) {
    world
        .get::<&mut Position>(entity.0)
        .expect("Could not find Position")
        .y = y;
}

pub fn npc_setmap(world: &mut World, entity: &crate::Entity, map: MapPosition) {
    world
        .get::<&mut Position>(entity.0)
        .expect("Could not find Position")
        .map = map;
}

pub fn npc_sethp(world: &mut World, entity: &crate::Entity, hp: i32) {
    world
        .get::<&mut Vitals>(entity.0)
        .expect("Could not find Position")
        .vital[VitalTypes::Hp as usize] = hp;
}
