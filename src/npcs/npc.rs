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
pub struct NpcMoves(#[derivative(Default(value = "Vec::new()"))] pub Vec<(Position, u8)>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcSpawnedZone(pub Option<usize>);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct NpcMovePos(pub Option<Position>);

#[inline(always)]
pub fn is_npc_same(from_entity: &crate::Entity, to_entity: &crate::Entity) -> bool {
    from_entity == to_entity
}

#[inline(always)]
pub fn npc_set_move_path(world: &mut World, entity: &crate::Entity, path: Vec<(Position, u8)>) {
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
pub fn set_npc_dir(world: &mut World, storage: &Storage, entity: &crate::Entity, dir: u8) {
    if world.get_or_panic::<Dir>(entity).0 != dir {
        world
            .get::<&mut Dir>(entity.0)
            .expect("Could not find Dir")
            .0 = dir;

        let _ = DataTaskToken::NpcDir(world.get_or_panic::<Position>(entity).map)
            .add_task(storage, &DirPacket::new(*entity, dir));
    }
}

#[inline(always)]
pub fn npc_swap_pos(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    pos: Position,
) -> Position {
    let oldpos = world.get_or_panic::<Position>(entity);
    if oldpos != pos {
        *world
            .get::<&mut Position>(entity.0)
            .expect("Could not find Position") = pos;

        let mut map = match storage.maps.get(&oldpos.map) {
            Some(map) => map,
            None => return oldpos,
        }
        .borrow_mut();
        map.remove_entity_from_grid(oldpos);
        map.add_entity_to_grid(pos);
    }
    oldpos
}

#[inline(always)]
pub fn npc_switch_maps(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    pos: Position,
) -> Position {
    let oldpos = world.get_or_panic::<Position>(entity);
    let mut map = match storage
        .maps
        .get(&world.get_or_panic::<Position>(entity).map)
    {
        Some(map) => map,
        None => return oldpos,
    }
    .borrow_mut();
    map.remove_npc(*entity);
    map.remove_entity_from_grid(world.get_or_panic::<Position>(entity));

    let mut map = match storage.maps.get(&pos.map) {
        Some(map) => map,
        None => return oldpos,
    }
    .borrow_mut();
    map.add_npc(*entity);
    map.add_entity_to_grid(pos);

    *world
        .get::<&mut Position>(entity.0)
        .expect("Could not find Position") = pos;
    oldpos
}

pub fn npc_getx(world: &mut World, entity: &crate::Entity) -> i32 {
    world.get_or_panic::<Position>(entity).x
}

pub fn npc_gety(world: &mut World, entity: &crate::Entity) -> i32 {
    world.get_or_panic::<Position>(entity).y
}

pub fn npc_getmap(world: &mut World, entity: &crate::Entity) -> MapPosition {
    world.get_or_panic::<Position>(entity).map
}

pub fn npc_gethp(world: &mut World, entity: &crate::Entity) -> i32 {
    world.get_or_panic::<Vitals>(entity).vital[VitalTypes::Hp as usize]
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

#[inline(always)]
pub fn damage_npc(world: &mut World, entity: &crate::Entity, damage: i32) {
    world
        .get::<&mut Vitals>(entity.0)
        .expect("Could not find Position")
        .vital[VitalTypes::Hp as usize] =
        world.get_or_panic::<Vitals>(entity).vital[VitalTypes::Hp as usize].saturating_sub(damage);
}
