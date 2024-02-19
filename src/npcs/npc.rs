use crate::{containers::*, gametypes::*, tasks::*, time_ext::MyInstant};
use unwrap_helpers::*;

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcIndex(#[derivative(Default(value = "0"))] pub u64);

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

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcSpawnedZone(#[derivative(Default(value = "None"))] pub Option<usize>);

#[derive(Derivative, Debug, Copy, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct NpcMovePos(#[derivative(Default(value = "None"))] pub Option<Position>);

#[inline(always)]
pub fn is_npc_same(from_entity: &crate::Entity, to_entity: &crate::Entity) -> bool {
    from_entity == to_entity
}

#[inline(always)]
pub fn npc_set_move_path(
    world: &mut hecs::World,
    entity: &crate::Entity,
    path: Vec<(Position, u8)>,
) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if let mut npcmoves = data
        .get::<&mut NpcMoves>()
        .expect("Could not find NpcMoves")
    {
        npcmoves.0 = path
    };
    if let mut npcmoving = data
        .get::<&mut NpcMoving>()
        .expect("Could not find NpcMoving")
    {
        npcmoving.0 = true
    };
}

#[inline(always)]
pub fn npc_clear_move_path(world: &mut hecs::World, entity: &crate::Entity) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if let mut npcmoves = data
        .get::<&mut NpcMoves>()
        .expect("Could not find NpcMoves")
    {
        npcmoves.0.clear()
    };
    if let mut npcmoving = data
        .get::<&mut NpcMoving>()
        .expect("Could not find NpcMoving")
    {
        npcmoving.0 = false
    };
}

#[inline(always)]
pub fn set_npc_dir(world: &mut hecs::World, storage: &Storage, entity: &crate::Entity, dir: u8) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if data.get::<&Dir>().expect("Could not find Dir").0 != dir {
        if let mut playerdir = data.get::<&mut Dir>().expect("Could not find Dir") {
            playerdir.0 = dir
        };

        let _ = DataTaskToken::NpcDir(
            data.get::<&Position>()
                .expect("Could not find Position")
                .map,
        )
        .add_task(storage, &DirPacket::new(*entity, dir));
    }
}

#[inline(always)]
pub fn npc_swap_pos(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &crate::Entity,
    pos: Position,
) -> Position {
    let data = world.entity(entity.0).expect("Could not get Entity");

    let oldpos = data.get::<&Position>().expect("Could not find Position");
    if *oldpos != pos {
        if let mut position = data
            .get::<&mut Position>()
            .expect("Could not find Position")
        {
            *position = pos
        };

        let mut map = unwrap_or_return!(storage.maps.get(&oldpos.map), *oldpos).borrow_mut();
        map.remove_entity_from_grid(*oldpos);
        map.add_entity_to_grid(pos);
    }

    *oldpos
}

#[inline(always)]
pub fn npc_switch_maps(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &crate::Entity,
    pos: Position,
) -> Position {
    let data = world.entity(entity.0).expect("Could not get Entity");

    let oldpos = data.get::<&Position>().expect("Could not find Position");
    let mut map = unwrap_or_return!(
        storage.maps.get(
            &data
                .get::<&Position>()
                .expect("Could not find Position")
                .map
        ),
        *oldpos
    )
    .borrow_mut();
    map.remove_npc(*entity);
    map.remove_entity_from_grid(*data.get::<&Position>().expect("Could not find Position"));

    let mut map = unwrap_or_return!(storage.maps.get(&pos.map), *oldpos).borrow_mut();
    map.add_npc(*entity);
    map.add_entity_to_grid(pos);

    if let mut position = data
        .get::<&mut Position>()
        .expect("Could not find Position")
    {
        *position = pos
    };
    *oldpos
}

pub fn npc_getx(world: &mut hecs::World, entity: &crate::Entity) -> i32 {
    let data = world.entity(entity.0).expect("Could not get Entity");

    data.get::<&Position>().expect("Could not find Position").x
}

pub fn npc_gety(world: &mut hecs::World, entity: &crate::Entity) -> i32 {
    let data = world.entity(entity.0).expect("Could not get Entity");

    data.get::<&Position>().expect("Could not find Position").y
}

pub fn npc_getmap(world: &mut hecs::World, entity: &crate::Entity) -> MapPosition {
    let data = world.entity(entity.0).expect("Could not get Entity");

    data.get::<&Position>()
        .expect("Could not find Position")
        .map
}

pub fn npc_gethp(world: &mut hecs::World, entity: &crate::Entity) -> i32 {
    let data = world.entity(entity.0).expect("Could not get Entity");

    data.get::<&Vitals>().expect("Could not find Vitals").vital[VitalTypes::Hp as usize]
}

pub fn npc_setx(world: &mut hecs::World, entity: &crate::Entity, x: i32) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if let mut position = data
        .get::<&mut Position>()
        .expect("Could not find Position")
    {
        position.x = x
    };
}

pub fn npc_sety(world: &mut hecs::World, entity: &crate::Entity, y: i32) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if let mut position = data
        .get::<&mut Position>()
        .expect("Could not find Position")
    {
        position.y = y
    };
}

pub fn npc_setmap(world: &mut hecs::World, entity: &crate::Entity, map: MapPosition) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if let mut position = data
        .get::<&mut Position>()
        .expect("Could not find Position")
    {
        position.map = map
    };
}

pub fn npc_sethp(world: &mut hecs::World, entity: &crate::Entity, hp: i32) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if let mut vitals = data.get::<&mut Vitals>().expect("Could not find Vitals") {
        vitals.vital[VitalTypes::Hp as usize] = hp
    };
}

#[inline(always)]
pub fn damage_npc(world: &mut hecs::World, entity: &crate::Entity, damage: i32) {
    let data = world.entity(entity.0).expect("Could not get Entity");

    if let mut vitals = data.get::<&mut Vitals>().expect("Could not find Vitals") {
        vitals.vital[VitalTypes::Hp as usize] =
            vitals.vital[VitalTypes::Hp as usize].saturating_sub(damage)
    };
}
