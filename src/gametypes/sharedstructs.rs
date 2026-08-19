use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    Default,
    Readable,
    Writable,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub struct TileBox {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    Eq,
    Readable,
    Writable,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub struct GameTime {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
}

impl GameTime {
    pub fn in_range(&self, from: GameTime, to: GameTime) -> bool {
        let maintime = self.get_time();

        maintime >= from.get_time() && maintime <= to.get_time()
    }

    pub fn get_time(&self) -> jiff::civil::Time {
        jiff::civil::time(self.hour as i8, self.min as i8, self.sec as i8, 0)
    }
}

pub fn get_dir_sides(dir: u8) -> [u8; 2] {
    match dir {
        0 | 2 => [1, 3],
        _ => [0, 2],
    }
}
