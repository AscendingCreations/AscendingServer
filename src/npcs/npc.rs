use crate::{containers::*, gametypes::*, tasks::*, time_ext::MyInstant};
use educe::Educe;
use std::collections::VecDeque;

#[inline(always)]
pub fn is_npc_same(from_entity: GlobalKey, to_entity: GlobalKey) -> bool {
    from_entity == to_entity
}

#[inline(always)]
pub fn npc_set_move_path(
    world: &mut World,
    entity: GlobalKey,
    path: VecDeque<(Position, u8)>,
) -> Result<()> {
    world.get::<&mut NpcMoves>(entity.0)?.0 = path;
    world.get::<&mut NpcMoving>(entity.0)?.0 = true;
    Ok(())
}

#[inline(always)]
pub fn npc_clear_move_path(world: &mut World, entity: GlobalKey) -> Result<()> {
    world.get::<&mut NpcMoves>(entity.0)?.0.clear();
    world.get::<&mut NpcMoving>(entity.0)?.0 = false;
    Ok(())
}

#[inline(always)]
pub fn set_npc_dir(world: &mut World, storage: &Storage, entity: GlobalKey, dir: u8) -> Result<()> {
    if world.get_or_err::<Dir>(entity)?.0 != dir {
        world.get::<&mut Dir>(entity.0)?.0 = dir;

        DataTaskToken::Dir(world.get_or_err::<Position>(entity)?.map)
            .add_task(storage, dir_packet(*entity, dir)?)?;
    }

    Ok(())
}

#[inline(always)]
pub fn npc_switch_maps(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    new_pos: Position,
) -> Result<Position> {
    let npc_position = world.get_or_err::<Position>(entity)?;

    if let Some(mapref) = storage.maps.get(&npc_position.map) {
        let mut map = mapref.borrow_mut();
        map.remove_npc(*entity);
        map.remove_entity_from_grid(npc_position);
    } else {
        return Ok(npc_position);
    }

    if let Some(mapref) = storage.maps.get(&new_pos.map) {
        let mut map = mapref.borrow_mut();
        map.add_npc(*entity);
        map.add_entity_to_grid(new_pos);
    } else {
        return Ok(npc_position);
    }

    *world.get::<&mut Position>(entity.0)? = new_pos;

    Ok(npc_position)
}

#[inline(always)]
pub fn npc_swap_pos(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    pos: Position,
) -> Result<Position> {
    let oldpos = world.get_or_err::<Position>(entity)?;
    if oldpos != pos {
        *world.get::<&mut Position>(entity.0)? = pos;

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

pub fn npc_getx(world: &mut World, entity: GlobalKey) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity)?.x)
}

pub fn npc_gety(world: &mut World, entity: GlobalKey) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity)?.y)
}

pub fn npc_getmap(world: &mut World, entity: GlobalKey) -> Result<MapPosition> {
    Ok(world.get_or_err::<Position>(entity)?.map)
}

pub fn npc_gethp(world: &mut World, entity: GlobalKey) -> Result<i32> {
    Ok(world.get_or_err::<Vitals>(entity)?.vital[VitalTypes::Hp as usize])
}

pub fn npc_setx(world: &mut World, entity: GlobalKey, x: i32) -> Result<()> {
    world.get::<&mut Position>(entity.0)?.x = x;
    Ok(())
}

pub fn npc_sety(world: &mut World, entity: GlobalKey, y: i32) -> Result<()> {
    world.get::<&mut Position>(entity.0)?.y = y;
    Ok(())
}

pub fn npc_setmap(world: &mut World, entity: GlobalKey, map: MapPosition) -> Result<()> {
    world.get::<&mut Position>(entity.0)?.map = map;
    Ok(())
}

pub fn npc_sethp(world: &mut World, entity: GlobalKey, hp: i32) -> Result<()> {
    world.get::<&mut Vitals>(entity.0)?.vital[VitalTypes::Hp as usize] = hp;
    Ok(())
}
