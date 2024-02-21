use crate::{
    containers::*, gameloop::*, gametypes::*, items::*, socket::*, sql::*, tasks::*, time_ext::*,
};
use bytey::{ByteBufferRead, ByteBufferWrite};
use hecs::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Bundle)]
pub struct Socket {
    // IP address
    pub addr: String,
    // Socket ID
    pub id: usize,
    // Packet Buffer
    pub buffer: ByteBuffer,
}

impl Socket {
    #[inline(always)]
    pub fn new(id: usize, addr: String) -> Result<Self> {
        Ok(Self {
            id,
            addr,
            buffer: ByteBuffer::with_capacity(8192)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub name: String,
    pub passresetcode: Option<String>,
    pub id: i64,
}

#[derive(Copy, Clone, Debug, Derivative)]
#[derivative(Default)]
pub struct PlayerItemTimer {
    #[derivative(Default(value = "MyInstant::now()"))]
    pub itemtimer: MyInstant,
}

#[derive(Copy, Clone, Debug, Derivative)]
#[derivative(Default)]
pub struct PlayerMapTimer {
    #[derivative(Default(value = "MyInstant::now()"))]
    pub mapitemtimer: MyInstant,
}

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub items: Vec<Item>,
}

#[derive(
    PartialEq, Eq, Clone, Debug, Default, Deserialize, Serialize, ByteBufferRead, ByteBufferWrite,
)]
pub struct Equipment {
    pub items: Vec<Item>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Sprite {
    pub id: u32,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Money {
    pub vals: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MapSwitchTasks {
    pub tasks: Vec<usize>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Player {
    pub levelexp: u64,
    pub useditemid: u32,
    pub resetcount: i16,
    pub pvpon: bool,
    pub pk: bool,
    pub movesavecount: u16,
}

pub fn is_player_online(world: &mut hecs::World, entity: &crate::Entity) -> bool {
    *world
        .get::<&WorldEntityType>(entity.0)
        .expect("Could not find WorldEntityType")
        == WorldEntityType::Player
        && *world
            .get::<&OnlineType>(entity.0)
            .expect("Could not find WorldEntityType")
            == OnlineType::Online
}

#[inline(always)]
pub fn player_switch_maps(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &crate::Entity,
    new_pos: Position,
) -> Position {
    let player_position = world.get_or_panic::<Position>(entity);

    let old_position = player_position;
    let mut map = match storage.maps.get(&player_position.map) {
        Some(map) => map,
        None => return old_position,
    }
    .borrow_mut();
    map.remove_player(storage, *entity);
    map.remove_entity_from_grid(player_position);

    let mut map = match storage.maps.get(&new_pos.map) {
        Some(map) => map,
        None => return old_position,
    }
    .borrow_mut();
    map.add_player(storage, *entity);
    map.add_entity_to_grid(new_pos);

    {
        *world
            .get::<&mut Position>(entity.0)
            .expect("Could not find Position") = new_pos;
    }
    old_position
}

#[inline(always)]
pub fn player_swap_pos(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &crate::Entity,
    pos: Position,
) -> Position {
    let mut query = world
        .query_one::<&mut Position>(entity.0)
        .expect("player_swap_pos could not find query");

    if let Some(player_position) = query.get() {
        let old_position = *player_position;

        if old_position != pos {
            *player_position = pos;

            let mut map = match storage.maps.get(&old_position.map) {
                Some(map) => map,
                None => return old_position,
            }
            .borrow_mut();
            map.remove_entity_from_grid(old_position);
            map.add_entity_to_grid(pos);
        }

        old_position
    } else {
        Position::default()
    }
}

pub fn player_add_up_vital(world: &mut hecs::World, entity: &crate::Entity, vital: usize) -> i32 {
    let mut query = world
        .query_one::<&mut Vitals>(entity.0)
        .expect("player_add_up_vital could not find query");

    if let Some(player_vital) = query.get() {
        let hp = player_vital.vitalmax[vital].saturating_add(player_vital.vitalbuffs[vital]);

        if hp.is_negative() || hp == 0 {
            1
        } else {
            hp
        }
    } else {
        1
    }
}

#[inline(always)]
pub fn player_set_dir(world: &mut hecs::World, storage: &Storage, entity: &crate::Entity, dir: u8) {
    let mut query = world
        .query_one::<(&mut Dir, &Position)>(entity.0)
        .expect("player_set_dir could not find query");

    if let Some((player_dir, player_position)) = query.get() {
        if player_dir.0 != dir {
            player_dir.0 = dir;

            let _ = DataTaskToken::PlayerDir(player_position.map)
                .add_task(storage, &DirPacket::new(*entity, dir));
        }
    }
}

pub fn player_getx(world: &mut hecs::World, entity: &crate::Entity) -> i32 {
    let mut query = world
        .query_one::<&Position>(entity.0)
        .expect("player_getx could not find query");

    if let Some(player_position) = query.get() {
        player_position.x
    } else {
        0
    }
}

pub fn player_gety(world: &mut hecs::World, entity: &crate::Entity) -> i32 {
    let mut query = world
        .query_one::<&Position>(entity.0)
        .expect("player_gety could not find query");

    if let Some(player_position) = query.get() {
        player_position.y
    } else {
        0
    }
}

pub fn player_getmap(world: &mut hecs::World, entity: &crate::Entity) -> MapPosition {
    let mut query = world
        .query_one::<&Position>(entity.0)
        .expect("player_getmap could not find query");

    if let Some(player_position) = query.get() {
        player_position.map
    } else {
        MapPosition::new(0, 0, 0)
    }
}

pub fn player_gethp(world: &mut hecs::World, entity: &crate::Entity) -> i32 {
    let mut query = world
        .query_one::<&Vitals>(entity.0)
        .expect("player_gethp could not find query");

    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize]
    } else {
        0
    }
}

pub fn player_setx(world: &mut hecs::World, entity: &crate::Entity, x: i32) {
    let mut query = world
        .query_one::<&mut Position>(entity.0)
        .expect("player_setx could not find query");

    if let Some(player_position) = query.get() {
        player_position.x = x;
    }
}

pub fn player_sety(world: &mut hecs::World, entity: &crate::Entity, y: i32) {
    let mut query = world
        .query_one::<&mut Position>(entity.0)
        .expect("player_sety could not find query");

    if let Some(player_position) = query.get() {
        player_position.y = y;
    }
}

pub fn player_setmap(world: &mut hecs::World, entity: &crate::Entity, map: MapPosition) {
    let mut query = world
        .query_one::<&mut Position>(entity.0)
        .expect("player_setmap could not find query");

    if let Some(player_position) = query.get() {
        player_position.map = map;
    }
}

pub fn player_sethp(world: &mut hecs::World, entity: &crate::Entity, hp: i32) {
    let mut query = world
        .query_one::<&mut Vitals>(entity.0)
        .expect("player_sethp could not find query");

    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize] = hp;
    }
}

#[inline]
pub fn damage_player(world: &mut hecs::World, entity: &crate::Entity, damage: i32) {
    let mut query = world
        .query_one::<&mut Vitals>(entity.0)
        .expect("damage_player could not find query");

    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize] =
            player_vital.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }
}

#[inline]
pub fn player_give_vals(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &crate::Entity,
    amount: u64,
) -> u64 {
    let player_money = world.get_or_panic::<&Money>(entity);
    let rem = u64::MAX.saturating_sub(player_money.vals);

    if rem > 0 {
        let mut cur = amount;
        if rem >= cur {
            {
                world
                    .get::<&mut Money>(entity.0)
                    .expect("Could not find Money")
                    .vals = world
                    .get_or_panic::<&Money>(entity)
                    .vals
                    .saturating_add(cur);
            }
            cur = 0;
        } else {
            {
                world
                    .get::<&mut Money>(entity.0)
                    .expect("Could not find Money")
                    .vals = u64::MAX;
            }
            cur = cur.saturating_sub(rem);
        }

        let _ = send_money(world, storage, entity);
        let _ = update_currency(&mut storage.pgconn.borrow_mut(), world, entity);
        let _ = send_fltalert(
            storage,
            world.get_or_panic::<&Socket>(entity).id,
            format!("You Have Received {} Vals.", amount - cur),
            FtlType::Money,
        );
        return cur;
    }

    amount
}

#[inline]
pub fn player_take_vals(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &crate::Entity,
    amount: u64,
) {
    let mut cur = amount;

    let player_money = world.get_or_panic::<&Money>(entity);
    if player_money.vals >= cur {
        {
            world
                .get::<&mut Money>(entity.0)
                .expect("Could not find Money")
                .vals = world
                .get_or_panic::<&Money>(entity)
                .vals
                .saturating_sub(cur);
        }
    } else {
        cur = player_money.vals;
        {
            world
                .get::<&mut Money>(entity.0)
                .expect("Could not find Money")
                .vals = 0;
        }
    }

    let _ = send_money(world, storage, entity);
    let _ = update_currency(&mut storage.pgconn.borrow_mut(), world, entity);
    let _ = send_fltalert(
        storage,
        world.get_or_panic::<&Socket>(entity).id,
        format!("You Lost {} Vals.", cur),
        FtlType::Money,
    );
}
