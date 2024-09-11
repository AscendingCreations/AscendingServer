use crate::{maps::MapBroadCasts, time_ext::MyInstant, Result};
use chrono::{Duration, NaiveTime};
use log::error;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use tokio::sync::broadcast;

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

//Keeps the GameTime Updated Across Maps.
pub struct GameTimeActor {
    pub time: GameTime,
    pub tx: broadcast::Sender<MapBroadCasts>,
}

impl GameTimeActor {
    pub fn new(tx: broadcast::Sender<MapBroadCasts>) -> Self {
        Self {
            time: GameTime::default(),
            tx,
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        let mut tick: MyInstant;
        let mut game_time_timer: MyInstant = MyInstant::now();

        loop {
            tick = MyInstant::now();

            if tick > game_time_timer {
                self.time.min += 1;
                if self.time.min >= 60 {
                    self.time.min = 0;
                    self.time.hour += 1;
                    if self.time.hour >= 24 {
                        self.time.hour = 0;
                    }
                }
                game_time_timer = tick + Duration::try_milliseconds(60000).unwrap_or_default();
                self.tx
                    .send(MapBroadCasts::TimeUpdate { time: self.time })?;
            }
        }
    }
}
