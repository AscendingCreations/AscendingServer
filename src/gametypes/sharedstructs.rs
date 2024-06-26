use chrono::NaiveTime;
use log::error;
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

    pub fn get_time(&self) -> NaiveTime {
        NaiveTime::from_hms_opt(self.hour, self.min, self.sec).unwrap_or_else(|| {
            error!(
                "gametime Hour:{}, Min:{} or second:{} is not being set correctly.",
                self.hour, self.min, self.sec
            );
            NaiveTime::default()
        })
    }
}

pub fn get_dir_sides(dir: u8) -> [u8; 2] {
    match dir {
        0 | 2 => [1, 3],
        _ => [0, 2],
    }
}
