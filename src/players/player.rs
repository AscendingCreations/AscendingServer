use std::sync::{Arc, Mutex};

use crate::{
    containers::*, gameloop::*, gametypes::*, items::*, socket::*, sql::*, tasks::*, time_ext::*,
};
use hecs::*;
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
    if let Ok(mut result) = 
        world.query_one::<&mut Spawn>(player.0) {
        if let Some(player_spawn) = result.get() {
            let old_spawn = player_spawn.clone();
            let mut map = unwrap_or_return!(storage.maps.get(&player_spawn.pos.map), old_spawn.pos).borrow_mut();
            map.remove_player(world, storage, *player);
            map.remove_entity_from_grid(player_spawn.pos);

            let mut map = unwrap_or_return!(storage.maps.get(&new_pos.map), old_spawn.pos).borrow_mut();
            map.add_player(world, storage, *player);
            map.add_entity_to_grid(new_pos);

            player_spawn.pos = new_pos;
            return old_spawn.pos;
        }
    }
    Position::default()
}

#[inline(always)]
pub fn player_swap_pos(world: &mut hecs::World, storage: &Storage, player: &crate::Entity, pos: Position) -> Position {
    if let Ok(mut result) = 
        world.query_one::<&mut Spawn>(player.0) {
        if let Some(player_spawn) = result.get() {

            let old_spawn = player_spawn.clone();

            if old_spawn.pos != pos {
                player_spawn.pos = pos;

                let mut map = unwrap_or_return!(storage.maps.get(&old_spawn.pos.map), old_spawn.pos).borrow_mut();
                map.remove_entity_from_grid(old_spawn.pos);
                map.add_entity_to_grid(pos);
            }

            return old_spawn.pos;
        }
    }
    Position::default()
}

pub fn player_add_up_vital(world: &mut hecs::World, player: &crate::Entity, vital: usize) -> i32 {
    if let Ok(mut result) = 
        world.query_one::<&mut Vitals>(player.0) {
        if let Some(player_vital) = result.get() {
            let hp = player_vital.vitalmax[vital].saturating_add(player_vital.vitalbuffs[vital]);

            if hp.is_negative() || hp == 0 {
                return 1;
            } else {
                return hp;
            }
        }
    }
    1
}

/*impl Player {
    #[inline(always)]
    pub fn set_dir(&mut self, world: &mut hecs::World, storage: &Storage, dir: u8) {
        if self.e.dir != dir {
            self.e.dir = dir;

            let _ = DataTaskToken::PlayerDir(self.e.pos.map)
                .add_task(world, &DirPacket::new(self.e.get_id() as u64, dir));
        }
    }

    #[inline(always)]
    pub fn swap_pos(&mut self, world: &mut hecs::World, storage: &Storage, pos: Position) -> Position {
        let oldpos = self.e.pos;
        if oldpos != pos {
            self.e.pos = pos;

            let mut map = unwrap_or_return!(world.maps.get(&oldpos.map), oldpos).borrow_mut();
            map.remove_entity_from_grid(oldpos);
            map.add_entity_to_grid(pos);
        }

        oldpos
    }

    #[inline(always)]
    pub fn switch_maps(&mut self, world: &mut hecs::World, storage: &Storage, pos: Position) -> Position {
        let oldpos = self.e.pos;
        let mut map = unwrap_or_return!(world.maps.get(&self.e.pos.map), oldpos).borrow_mut();
        map.remove_player(world, self.e.get_id());
        map.remove_entity_from_grid(self.e.pos);

        let mut map = unwrap_or_return!(world.maps.get(&pos.map), oldpos).borrow_mut();
        map.add_player(world, self.e.get_id());
        map.add_entity_to_grid(pos);

        self.e.pos = pos;
        oldpos
    }

    pub fn getx(&self) -> i32 {
        self.e.pos.x
    }

    pub fn gety(&self) -> i32 {
        self.e.pos.y
    }

    pub fn getmap(&self) -> MapPosition {
        self.e.pos.map
    }

    pub fn gethp(&self) -> i32 {
        self.e.vital[VitalTypes::Hp as usize]
    }

    pub fn setx(&mut self, x: i32) {
        self.e.pos.x = x;
    }

    pub fn sety(&mut self, y: i32) {
        self.e.pos.y = y;
    }

    pub fn setmap(&mut self, map: MapPosition) {
        self.e.pos.map = map;
    }

    pub fn sethp(&mut self, hp: i32) {
        self.e.vital[VitalTypes::Hp as usize] = hp;
    }

    #[inline]
    pub fn damage_player(&mut self, damage: i32) {
        self.e.vital[VitalTypes::Hp as usize] =
            self.e.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }
}*/

/*
#[inline]
pub fn give_vals(world: &Storage, user: &mut Player, amount: u64) -> u64 {
    let rem = u64::MAX.saturating_sub(user.vals);

    if rem > 0 {
        let mut cur = amount;
        if rem >= cur {
            user.vals = user.vals.saturating_add(cur);
            cur = 0;
        } else {
            user.vals = u64::MAX;
            cur = cur.saturating_sub(rem);
        }

        let _ = send_money(world, user);
        let _ = update_currency(&mut world.pgconn.borrow_mut(), user);
        let _ = send_fltalert(
            world,
            user.socket_id,
            format!("You Have Received {} Vals.", amount - cur),
            FtlType::Money,
        );
        return cur;
    }

    amount
}

#[inline]
pub fn take_vals(world: &Storage, user: &mut Player, amount: u64) {
    let mut cur = amount;

    if user.vals >= cur {
        user.vals = user.vals.saturating_sub(cur);
    } else {
        cur = user.vals;
        user.vals = 0;
    }

    let _ = send_money(world, user);
    let _ = update_currency(&mut world.pgconn.borrow_mut(), user);
    let _ = send_fltalert(
        world,
        user.socket_id,
        format!("You Lost {} Vals.", cur),
        FtlType::Money,
    );
}*/
