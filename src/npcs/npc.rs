use crate::{containers::*, gametypes::*, tasks::*, time_ext::MyInstant};
use educe::Educe;
use hecs::World;
use std::collections::VecDeque;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct NpcIndex(pub u64);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcTimer {
    #[educe(Default = MyInstant::now())]
    pub despawntimer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub spawntimer: MyInstant,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcAITimer(#[educe(Default = MyInstant::now())] pub MyInstant); //for rebuilding the a* paths

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcPathTimer {
    #[educe(Default = MyInstant::now())]
    pub timer: MyInstant,
    pub tries: usize,
    //when failing to move due to blocks in movement.
    pub fails: usize,
} //for rebuilding the a* paths

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcDespawns(#[educe(Default = false)] pub bool);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcMoving(#[educe(Default = false)] pub bool);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcRetreating(#[educe(Default = false)] pub bool);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcWalkToSpawn(#[educe(Default = false)] pub bool);

#[derive(Educe, Debug, Clone, PartialEq, Eq)]
#[educe(Default)]
//offset for special things so the npc wont to events based on this spawn time.
pub struct NpcHitBy(#[educe(Default = Vec::new())] pub Vec<(u32, u64, u64)>);

#[derive(Educe, Debug, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct NpcMoves(#[educe(Default = VecDeque::new())] pub VecDeque<(Position, u8)>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcSpawnedZone(pub Option<usize>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcMovePos(pub Option<Position>);

#[inline(always)]
pub fn is_npc_same(from_entity: &crate::Entity, to_entity: &crate::Entity) -> bool {
    from_entity == to_entity
}

#[inline(always)]
pub async fn npc_set_move_path(
    world: &mut World,
    entity: &crate::Entity,
    path: VecDeque<(Position, u8)>,
) -> Result<()> {
    world.get::<&mut NpcMoves>(entity.0)?.0 = path;
    world.get::<&mut NpcMoving>(entity.0)?.0 = true;
    Ok(())
}

#[inline(always)]
pub async fn npc_clear_move_path(world: &mut World, entity: &crate::Entity) -> Result<()> {
    world.get::<&mut NpcMoves>(entity.0)?.0.clear();
    world.get::<&mut NpcMoving>(entity.0)?.0 = false;
    Ok(())
}

#[inline(always)]
pub async fn set_npc_dir(
    world: &mut World,
    storage: &GameStore,
    entity: &crate::Entity,
    dir: u8,
) -> Result<()> {
    if world.get_or_err::<Dir>(entity)?.0 != dir {
        world.get::<&mut Dir>(entity.0)?.0 = dir;

        DataTaskToken::Dir(world.get_or_err::<Position>(entity)?.map)
            .add_task(storage, dir_packet(*entity, dir)?)
            .await?;
    }

    Ok(())
}

#[inline(always)]
pub async fn npc_switch_maps(
    world: &mut World,
    storage: &GameStore,
    entity: &crate::Entity,
    new_pos: Position,
) -> Result<Position> {
    let npc_position = world.get_or_err::<Position>(entity)?;

    if let Some(mapref) = storage.maps.get(&npc_position.map) {
        let mut map = mapref.lock().await;
        map.remove_npc(*entity);
        map.remove_entity_from_grid(npc_position);
    } else {
        return Ok(npc_position);
    }

    if let Some(mapref) = storage.maps.get(&new_pos.map) {
        let mut map = mapref.lock().await;
        map.add_npc(*entity);
        map.add_entity_to_grid(new_pos);
    } else {
        return Ok(npc_position);
    }

    *world.get::<&mut Position>(entity.0)? = new_pos;

    Ok(npc_position)
}

#[inline(always)]
pub async fn npc_swap_pos(
    world: &mut World,
    storage: &GameStore,
    entity: &crate::Entity,
    pos: Position,
) -> Result<Position> {
    let oldpos = world.get_or_err::<Position>(entity)?;
    if oldpos != pos {
        *world.get::<&mut Position>(entity.0)? = pos;

        let mut map = match storage.maps.get(&oldpos.map) {
            Some(map) => map,
            None => return Ok(oldpos),
        }
        .lock()
        .await;
        map.remove_entity_from_grid(oldpos);
        map.add_entity_to_grid(pos);
    }

    Ok(oldpos)
}

pub async fn npc_getx(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity)?.x)
}

pub async fn npc_gety(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity)?.y)
}

pub async fn npc_getmap(world: &mut World, entity: &crate::Entity) -> Result<MapPosition> {
    Ok(world.get_or_err::<Position>(entity)?.map)
}

pub async fn npc_gethp(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Vitals>(entity)?.vital[VitalTypes::Hp as usize])
}

pub async fn npc_setx(world: &mut World, entity: &crate::Entity, x: i32) -> Result<()> {
    world.get::<&mut Position>(entity.0)?.x = x;
    Ok(())
}

pub async fn npc_sety(world: &mut World, entity: &crate::Entity, y: i32) -> Result<()> {
    world.get::<&mut Position>(entity.0)?.y = y;
    Ok(())
}

pub async fn npc_setmap(world: &mut World, entity: &crate::Entity, map: MapPosition) -> Result<()> {
    world.get::<&mut Position>(entity.0)?.map = map;
    Ok(())
}

pub async fn npc_sethp(world: &mut World, entity: &crate::Entity, hp: i32) -> Result<()> {
    world.get::<&mut Vitals>(entity.0)?.vital[VitalTypes::Hp as usize] = hp;
    Ok(())
}
