use crate::{
    containers::*, gameloop::*, gametypes::*, items::*, players::*, socket::*, sql::*, time_ext::*,
};
use unwrap_helpers::*;

#[derive(Clone, Debug, Derivative)]
#[derivative(Default(new = "true"))]
pub struct Player {
    pub e: Entity,
    pub name: String,
    pub addr: String,
    pub passresetcode: Option<String>,
    pub accid: i64,
    pub levelexp: u64,
    pub vals: u64,
    pub data: [i64; 5],
    pub socket_id: usize,
    pub useditemid: u32,
    #[derivative(Default(value = "Position::new(10, 10, MapPosition::new(0,0,0))"))]
    pub spawn: Position,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub itemtimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub mapitemtimer: MyInstant,
    pub access: UserAccess,
    pub achievements: Achievements,
    pub using: IsUsingType,
    pub status: OnlineType,
    #[derivative(Default(value = "ByteBuffer::with_capacity(8192).unwrap()"))]
    pub buffer: ByteBuffer,
    #[derivative(Default(value = "[Item::default(); MAX_INV]"))]
    pub inv: [Item; MAX_INV],
    pub equip: [Item; EQUIPMENT_TYPE_MAX],
    pub resetcount: i16,
    pub sprite: u8,
    pub pvpon: bool,
    pub pk: bool,
    pub movesavecount: u16,
    pub datatasks: Vec<usize>,
}

impl Player {
    #[inline(always)]
    pub fn set_dir(&mut self, world: &Storage, dir: u8) {
        if self.e.dir != dir {
            self.e.dir = dir;
            if let Err(i) = send_dir(world, self, false) {
                println!("{:?}", i);
            }
        }
    }

    #[inline(always)]
    pub fn swap_pos(&mut self, world: &Storage, pos: Position) -> Position {
        let oldpos = self.e.pos;
        if oldpos != pos {
            self.e.pos = pos;

            let mut map = unwrap_or_return!(world.map_data.get(&oldpos.map), oldpos).borrow_mut();
            map.remove_entity_from_grid(oldpos);
            map.add_entity_to_grid(pos);
        }

        oldpos
    }

    #[inline(always)]
    pub fn switch_maps(&mut self, world: &Storage, pos: Position) -> Position {
        let oldpos = self.e.pos;
        let mut map = unwrap_or_return!(world.map_data.get(&self.e.pos.map), oldpos).borrow_mut();
        map.remove_player(world, self.e.get_id());
        map.remove_entity_from_grid(self.e.pos);

        let mut map = unwrap_or_return!(world.map_data.get(&pos.map), oldpos).borrow_mut();
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
}

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
}
