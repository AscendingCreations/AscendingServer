use std::sync::{Arc, Mutex};

use crate::{
    containers::*, gameloop::*, gametypes::*, items::*, socket::*, sql::*, tasks::*, time_ext::*,
};
use hecs::*;
use phf::Map;
use unwrap_helpers::*;

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

#[derive(Clone, Debug, Derivative)]
pub struct PlayerItemTimer {
    #[derivative(Default(value = "MyInstant::now()"))]
    pub itemtimer: MyInstant,
}

#[derive(Clone, Debug, Derivative)]
pub struct PlayerMapTimer {
    #[derivative(Default(value = "MyInstant::now()"))]
    pub mapitemtimer: MyInstant,
}

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Default)]
pub struct Equipment {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Default)]
pub struct Sprite {
    pub id: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Money {
    pub vals: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MapSwitchTasks {
    pub tasks: Vec<usize>,
}

#[derive(Clone, Debug, Derivative)]
#[derivative(Default)]
pub struct Player {
    pub levelexp: u64,
    pub useditemid: u32,
    pub resetcount: i16,
    pub pvpon: bool,
    pub pk: bool,
    pub movesavecount: u16,
}

#[inline(always)]
pub fn player_switch_maps(world: &mut hecs::World, storage: &Storage, player: &crate::Entity, new_pos: Position) -> Position {
    let mut query = 
        world.query_one::<&mut Position>(player.0).expect("player_switch_maps could not find query");
    
    if let Some(player_position) = query.get() {
        let old_position = player_position.clone();
        let mut map = unwrap_or_return!(storage.maps.get(&player_position.map), old_position).borrow_mut();
        map.remove_player(world, storage, *player);
        map.remove_entity_from_grid(player_position.clone());

        let mut map = unwrap_or_return!(storage.maps.get(&new_pos.map), old_position).borrow_mut();
        map.add_player(world, storage, *player);
        map.add_entity_to_grid(new_pos);

        *player_position = new_pos;
        old_position
    } else {
        Position::default()
    }
}

#[inline(always)]
pub fn player_swap_pos(world: &mut hecs::World, storage: &Storage, player: &crate::Entity, pos: Position) -> Position {
    let mut query = 
        world.query_one::<&mut Position>(player.0).expect("player_swap_pos could not find query");
    
    if let Some(player_position) = query.get() {
        let old_position = player_position.clone();

        if old_position != pos {
            *player_position = pos;

            let mut map = unwrap_or_return!(storage.maps.get(&old_position.map), old_position).borrow_mut();
            map.remove_entity_from_grid(old_position);
            map.add_entity_to_grid(pos);
        }

        old_position
    } else {
        Position::default()
    }
}

pub fn player_add_up_vital(world: &mut hecs::World, player: &crate::Entity, vital: usize) -> i32 {
    let mut query = 
        world.query_one::<&mut Vitals>(player.0).expect("player_add_up_vital could not find query");
    
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
pub fn player_set_dir(world: &mut hecs::World, storage: &Storage, player: &crate::Entity, dir: u8) {
    let mut query = 
        world.query_one::<(&mut Dir, &Position)>(player.0).expect("player_set_dir could not find query");
    
    if let Some((player_dir, player_position)) = query.get() {
        if player_dir.0 != dir {
            player_dir.0 = dir;

            let _ = DataTaskToken::PlayerDir(player_position.map)
                .add_task(world, storage, &DirPacket::new(player, dir));
        }
    }
}

pub fn player_getx(world: &mut hecs::World, player: &crate::Entity) -> i32 {
    let mut query = 
        world.query_one::<&Position>(player.0).expect("player_getx could not find query");
    
    if let Some(player_position) = query.get() {
        player_position.x
    } else {
        0
    }
}

pub fn player_gety(world: &mut hecs::World, player: &crate::Entity) -> i32 {
    let mut query = 
        world.query_one::<&Position>(player.0).expect("player_gety could not find query");
    
    if let Some(player_position) = query.get() {
        player_position.y
    } else {
        0
    }
}

pub fn player_getmap(world: &mut hecs::World, player: &crate::Entity) -> MapPosition {
    let mut query = 
        world.query_one::<&Position>(player.0).expect("player_getmap could not find query");
    
    if let Some(player_position) = query.get() {
        player_position.map
    } else {
        MapPosition::new(0, 0, 0)
    }
}

pub fn player_gethp(world: &mut hecs::World, player: &crate::Entity) -> i32 {
    let mut query = 
        world.query_one::<&Vitals>(player.0).expect("player_gethp could not find query");
    
    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize]
    } else {
        0
    }
}

pub fn player_setx(world: &mut hecs::World, player: &crate::Entity, x: i32) {
    let mut query = 
        world.query_one::<&mut Position>(player.0).expect("player_setx could not find query");
    
    if let Some(player_position) = query.get() {
        player_position.x = x;
    }
}

pub fn player_sety(world: &mut hecs::World, player: &crate::Entity, y: i32) {
    let mut query = 
        world.query_one::<&mut Position>(player.0).expect("player_sety could not find query");
    
    if let Some(player_position) = query.get() {
        player_position.y = y;
    }
}

pub fn player_setmap(world: &mut hecs::World, player: &crate::Entity, map: MapPosition) {
    let mut query = 
        world.query_one::<&mut Position>(player.0).expect("player_setmap could not find query");
    
    if let Some(player_position) = query.get() {
        player_position.map = map;
    }
}

pub fn player_sethp(world: &mut hecs::World, player: &crate::Entity, hp: i32) {
    let mut query = 
        world.query_one::<&mut Vitals>(player.0).expect("player_sethp could not find query");
    
    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize] = hp;
    }
}

#[inline]
pub fn damage_player(world: &mut hecs::World, player: &crate::Entity, damage: i32) {
    let mut query = 
        world.query_one::<&mut Vitals>(player.0).expect("damage_player could not find query");
    
    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize] =
            player_vital.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }
}

#[inline]
pub fn player_give_vals(world: &mut hecs::World, storage: &Storage, player: &crate::Entity, amount: u64) -> u64 {
    let mut query = 
        world.query_one::<(&mut Money, &Socket)>(player.0).expect("player_give_vals could not find query");

    if let Some((player_money, socket)) = query.get() {
        let rem = u64::MAX.saturating_sub(player_money.vals);

        if rem > 0 {
            let mut cur = amount;
            if rem >= cur {
                player_money.vals = player_money.vals.saturating_add(cur);
                cur = 0;
            } else {
                player_money.vals = u64::MAX;
                cur = cur.saturating_sub(rem);
            }

            let _ = send_money(world, player);
            let _ = update_currency(&mut storage.pgconn.borrow_mut(), player);
            let _ = send_fltalert(
                world,
                socket.id,
                format!("You Have Received {} Vals.", amount - cur),
                FtlType::Money,
            );
            return cur;
        }

        amount
    } else {
        0
    }
}

#[inline]
pub fn player_take_vals(world: &mut hecs::World, storage: &Storage, player: &crate::Entity, amount: u64) {
    let mut query = 
        world.query_one::<(&mut Money, &Socket)>(player.0).expect("player_take_vals could not find query");

    if let Some((player_money, socket)) = query.get() {
        let mut cur = amount;

        if player_money.vals >= cur {
            player_money.vals = player_money.vals.saturating_sub(cur);
        } else {
            cur = player_money.vals;
            player_money.vals = 0;
        }

        let _ = send_money(world, player);
        let _ = update_currency(&mut storage.pgconn.borrow_mut(), player);
        let _ = send_fltalert(
            world,
            socket.id,
            format!("You Lost {} Vals.", cur),
            FtlType::Money,
        );
    }
}