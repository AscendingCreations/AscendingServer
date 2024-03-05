use crate::gametypes::*;
use bytey::{ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Deserialize,
    Serialize,
    Hash,
    ByteBufferRead,
    ByteBufferWrite,
    sqlx::Type,
)]
#[sqlx(type_name = "MapPosition")]
pub struct MapPosition {
    pub x: i32,
    pub y: i32,
    pub group: i32,
}

impl MapPosition {
    #[inline(always)]
    pub fn new(x: i32, y: i32, group: i32) -> MapPosition {
        MapPosition { x, y, group }
    }

    pub fn checkdistance(&self, target: MapPosition) -> i32 {
        if self.group != target.group {
            return 5000; //some big number here to prevent out of Group checks.
        }

        let x = self.x - target.x;
        let y = self.y - target.y;

        if x == 0 {
            return y.abs();
        }
        if y == 0 {
            return x.abs();
        }

        x.abs() + y.abs() - 1
    }

    pub fn map_offset(&self, dir: MapPosDir) -> MapPosition {
        match dir {
            MapPosDir::UpLeft => MapPosition::new(self.x + 1, self.y + 1, self.group),
            MapPosDir::Up => MapPosition::new(self.x, self.y + 1, self.group),
            MapPosDir::UpRight => MapPosition::new(self.x - 1, self.y + 1, self.group),
            MapPosDir::Left => MapPosition::new(self.x + 1, self.y, self.group),
            MapPosDir::None | MapPosDir::Center => MapPosition::new(self.x, self.y, self.group),
            MapPosDir::Right => MapPosition::new(self.x - 1, self.y, self.group),
            MapPosDir::DownLeft => MapPosition::new(self.x + 1, self.y - 1, self.group),
            MapPosDir::Down => MapPosition::new(self.x, self.y - 1, self.group),
            MapPosDir::DownRight => MapPosition::new(self.x - 1, self.y - 1, self.group),
        }
    }
}
