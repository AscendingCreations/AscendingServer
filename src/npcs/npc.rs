use crate::{containers::*, gametypes::*, tasks::*, time_ext::MyInstant};
use educe::Educe;
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq, Educe)]
#[educe(Default)]
pub struct Npc {
    pub index: u64,
    pub key: EntityKey,
    #[educe(Default = MyInstant::now())]
    pub spawn_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub ai_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub path_timer: MyInstant,
    pub path_tries: usize,
    //when failing to move due to blocks in movement.
    pub path_fails: usize,
    #[educe(Default = false)]
    pub despawn: bool,
    #[educe(Default = false)]
    pub moving: bool,
    #[educe(Default = false)]
    pub retreating: bool,
    #[educe(Default = VecDeque::new())]
    pub npc_moves: VecDeque<(Position, u8)>,
    pub spawn_zone: Option<usize>,
    pub spawn_map: MapPosition,
    pub move_pos_overide: Option<Position>,
    #[educe(Default = Position::new(10, 10, MapPosition::new(0,0,0)))]
    pub spawn_pos: Position,
    #[educe(Default  = MyInstant::now())]
    pub just_spawned: MyInstant,
    pub target: Targeting,
    pub kill_count: u32,
    #[educe(Default = MyInstant::now())]
    pub kill_count_timer: MyInstant,
    #[educe(Default = [25, 2, 100])]
    pub vital: [i32; VITALS_MAX],
    #[educe(Default = [25, 2, 100])]
    pub vitalmax: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub regens: [u32; VITALS_MAX],
    pub dir: u8,
    #[educe(Default = MyInstant::now())]
    pub despawn_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub attack_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub death_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub move_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub combat_timer: MyInstant,
    pub damage: u32,
    pub defense: u32,
    pub data: [i64; 10],
    pub hidden: bool,
    pub stunned: bool,
    pub attacking: bool,
    pub in_combat: bool,
    #[educe(Default = 1)]
    pub level: i32,
    pub position: Position,
    pub access: UserAccess,
    pub death_type: Death,
    pub is_using: IsUsingType,
}
/*
#[inline(always)]
pub fn is_npc_same(from_entity: &crate::Entity, to_entity: &crate::Entity) -> bool {
    from_entity == to_entity
}

#[inline(always)]
pub async fn npc_set_move_path(
    world: &GameWorld,
    entity: &crate::Entity,
    path: VecDeque<(Position, u8)>,
) -> Result<()> {
    let lock = world.write().await;
    lock.get::<&mut NpcMoves>(entity.0)?.0 = path;
    lock.get::<&mut NpcMoving>(entity.0)?.0 = true;
    Ok(())
}

#[inline(always)]
pub async fn npc_clear_move_path(world: &GameWorld, entity: &crate::Entity) -> Result<()> {
    let lock = world.write().await;
    lock.get::<&mut NpcMoves>(entity.0)?.0.clear();
    lock.get::<&mut NpcMoving>(entity.0)?.0 = false;
    Ok(())
}

#[inline(always)]
pub async fn set_npc_dir(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    dir: u8,
) -> Result<()> {
    if world.get_or_err::<Dir>(entity).await?.0 != dir {
        {
            let lock = world.write().await;
            lock.get::<&mut Dir>(entity.0)?.0 = dir;
        }

        DataTaskToken::Dir(world.get_or_err::<Position>(entity).await?.map)
            .add_task(storage, dir_packet(*entity, dir)?)
            .await?;
    }

    Ok(())
}

#[inline(always)]
pub async fn npc_switch_maps(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    new_pos: Position,
) -> Result<Position> {
    let npc_position = world.get_or_err::<Position>(entity).await?;

    if let Some(mapref) = storage.maps.get(&npc_position.map) {
        let mut map = mapref.write().await;
        map.remove_npc(*entity);
        map.remove_entity_from_grid(npc_position);
    } else {
        return Ok(npc_position);
    }

    if let Some(mapref) = storage.maps.get(&new_pos.map) {
        let mut map = mapref.write().await;
        map.add_npc(*entity);
        map.add_entity_to_grid(new_pos);
    } else {
        return Ok(npc_position);
    }

    let lock = world.write().await;
    *lock.get::<&mut Position>(entity.0)? = new_pos;

    Ok(npc_position)
}

#[inline(always)]
pub async fn npc_swap_pos(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    pos: Position,
) -> Result<Position> {
    let oldpos = world.get_or_err::<Position>(entity).await?;
    if oldpos != pos {
        let lock = world.write().await;
        *lock.get::<&mut Position>(entity.0)? = pos;

        let mut map = match storage.maps.get(&oldpos.map) {
            Some(map) => map,
            None => return Ok(oldpos),
        }
        .write()
        .await;
        map.remove_entity_from_grid(oldpos);
        map.add_entity_to_grid(pos);
    }

    Ok(oldpos)
}

pub async fn npc_getx(world: &GameWorld, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity).await?.x)
}

pub async fn npc_gety(world: &GameWorld, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Position>(entity).await?.y)
}

pub async fn npc_getmap(world: &GameWorld, entity: &crate::Entity) -> Result<MapPosition> {
    Ok(world.get_or_err::<Position>(entity).await?.map)
}

pub async fn npc_gethp(world: &GameWorld, entity: &crate::Entity) -> Result<i32> {
    Ok(world.get_or_err::<Vitals>(entity).await?.vital[VitalTypes::Hp as usize])
}

pub async fn npc_setx(world: &GameWorld, entity: &crate::Entity, x: i32) -> Result<()> {
    let lock = world.write().await;
    lock.get::<&mut Position>(entity.0)?.x = x;
    Ok(())
}

pub async fn npc_sety(world: &GameWorld, entity: &crate::Entity, y: i32) -> Result<()> {
    let lock = world.write().await;
    lock.get::<&mut Position>(entity.0)?.y = y;
    Ok(())
}

pub async fn npc_setmap(world: &GameWorld, entity: &crate::Entity, map: MapPosition) -> Result<()> {
    let lock = world.write().await;
    lock.get::<&mut Position>(entity.0)?.map = map;
    Ok(())
}

pub async fn npc_sethp(world: &GameWorld, entity: &crate::Entity, hp: i32) -> Result<()> {
    let lock = world.write().await;
    lock.get::<&mut Vitals>(entity.0)?.vital[VitalTypes::Hp as usize] = hp;
    Ok(())
}*/
