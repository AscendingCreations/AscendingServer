use crate::{gametypes::*, sql::*};
use std::convert::TryInto;

#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[table_name = "achievements"]
#[primary_key(uid)]
pub struct PGAchievements {
    uid: i64,
    daykills: i32,
    nightkills: i32,
    survivekill: i32,
    revivals: i32,
    deaths: i32,
    npckilled: Vec<i32>,
}

impl PGAchievements {
    pub fn new(achieve: &crate::players::Achievements, uid: i64) -> PGAchievements {
        PGAchievements {
            uid,
            daykills: achieve.daykills as i32,
            nightkills: achieve.nightkills as i32,
            survivekill: achieve.survivekill as i32,
            revivals: achieve.revivals as i32,
            deaths: achieve.deaths as i32,
            npckilled: achieve.npckilled.iter().map(|x| *x as i32).collect(),
        }
    }

    pub fn into_achievements(self, achieve: &mut crate::players::Achievements) {
        achieve.daykills = self.daykills as u32;
        achieve.nightkills = self.nightkills as u32;
        achieve.survivekill = self.survivekill as u32;
        achieve.revivals = self.revivals as u32;
        achieve.deaths = self.deaths as u32;
        achieve.npckilled = self
            .npckilled
            .iter()
            .map(|x| *x as u32)
            .collect::<Vec<u32>>()[..MAX_NPCS]
            .try_into()
            .unwrap_or([0; MAX_NPCS]);
    }
}
