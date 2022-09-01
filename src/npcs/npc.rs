use crate::{containers::*, gametypes::*, time_ext::MyInstant};
use unwrap_helpers::*;

#[derive(Clone, Debug, Derivative)]
#[derivative(Default(new = "true"))]
pub struct Npc {
    pub num: u64,
    pub sprite: u32,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub despawntimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub spawntimer: MyInstant,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub ai_timer: MyInstant, //for rebuilding the a* paths
    //offset for special things so the npc wont to events based on this spawn time.
    pub e: Entity,
    pub despawns: bool,
    pub hitby: Vec<(u32, u64, u64)>,
    //New pos and walking dir.
    pub moves: Vec<(Position, u8)>,
    pub moving: bool,
    pub is_retreating: bool,
    pub walktospawn: bool,
    //Zone ID so when they Die that map can spawn more.
    pub spawned_zone: Option<usize>,
    pub move_pos: Option<Position>,
}

impl Npc {
    #[inline(always)]
    pub fn is_same(&self, id: usize) -> bool {
        id == self.e.get_id()
    }

    #[inline(always)]
    pub fn set_move_path(&mut self, path: Vec<(Position, u8)>) {
        self.moves = path;
        self.moving = true;
    }

    #[inline(always)]
    pub fn clear_move_path(&mut self) {
        self.moves.clear();
        self.moving = false;
    }

    #[inline(always)]
    pub fn set_npc_dir(&mut self, _world: &Storage, dir: u8) {
        if self.e.dir != dir {
            self.e.dir = dir;
            //TODO: send dir turn to players.
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

    //TODO: Update to use MAP (x,y,group)
    #[inline(always)]
    pub fn switch_maps(&mut self, world: &Storage, pos: Position) -> Position {
        let oldpos = self.e.pos;
        let mut map = unwrap_or_return!(world.map_data.get(&self.e.pos.map), oldpos).borrow_mut();
        map.remove_npc(self.e.get_id());
        map.remove_entity_from_grid(self.e.pos);

        let mut map = unwrap_or_return!(world.map_data.get(&pos.map), oldpos).borrow_mut();
        map.add_npc(self.e.get_id());
        map.add_entity_to_grid(pos);

        self.e.pos = pos;
        oldpos
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

    #[inline(always)]
    pub fn damage_npc(&mut self, damage: i32) {
        self.e.vital[VitalTypes::Hp as usize] =
            self.e.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }
}
