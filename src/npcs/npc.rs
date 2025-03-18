use crate::{containers::*, gametypes::*, tasks::*};
use std::collections::VecDeque;

pub fn is_npc_same(from_entity: GlobalKey, to_entity: GlobalKey) -> bool {
    from_entity == to_entity
}

pub fn npc_set_move_path(
    world: &mut World,
    entity: GlobalKey,
    path: VecDeque<(Position, u8)>,
) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        n_data.moves.0 = path;
        n_data.moving = true;
    }
    Ok(())
}

pub fn npc_clear_move_path(world: &mut World, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        n_data.moves.0.clear();
        n_data.moving = false;
    }
    Ok(())
}

pub fn set_npc_dir(world: &mut World, storage: &Storage, entity: GlobalKey, dir: u8) -> Result<()> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        if n_data.movement.dir != dir {
            n_data.movement.dir = dir;

            DataTaskToken::Dir(n_data.movement.pos.map)
                .add_task(storage, dir_packet(entity, dir)?)?;
        }
    }

    Ok(())
}

pub fn npc_switch_maps(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    new_pos: Position,
) -> Result<Position> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        if let Some(mapref) = storage.maps.get(&n_data.movement.pos.map) {
            let mut map = mapref.borrow_mut();
            map.remove_npc(entity);
            map.remove_entity_from_grid(n_data.movement.pos);
        } else {
            return Ok(n_data.movement.pos);
        }

        if let Some(mapref) = storage.maps.get(&new_pos.map) {
            let mut map = mapref.borrow_mut();
            map.add_npc(entity);
            map.add_entity_to_grid(new_pos);
        } else {
            return Ok(n_data.movement.pos);
        }

        n_data.movement.pos = new_pos;

        Ok(n_data.movement.pos)
    } else {
        Ok(Position::default())
    }
}

pub fn npc_swap_pos(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    pos: Position,
) -> Result<Position> {
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let mut n_data = n_data.try_lock()?;

        let oldpos = n_data.movement.pos;

        if oldpos != pos {
            n_data.movement.pos = pos;

            let mut map = match storage.maps.get(&oldpos.map) {
                Some(map) => map,
                None => return Ok(oldpos),
            }
            .borrow_mut();

            map.remove_entity_from_grid(oldpos);
            map.add_entity_to_grid(pos);
        }

        Ok(oldpos)
    } else {
        Ok(Position::default())
    }
}
