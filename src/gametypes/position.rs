use crate::{containers::*, gametypes::*, maps::*};
use bytey::{ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};
use sqlx::Postgres;

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
)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub map: MapPosition,
}

impl sqlx::Type<Postgres> for Position {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("location")
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        *ty == Self::type_info()
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for Position {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> sqlx::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let x = decoder.try_decode::<i32>()?;
        let y = decoder.try_decode::<i32>()?;
        let map = decoder.try_decode::<MapPosition>()?;
        Ok(Self { x, y, map })
    }
}

impl<'q> sqlx::Encode<'q, Postgres> for Position {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder
            .encode(self.x)
            .encode(self.y)
            .encode(self.map)
            .finish();
        sqlx::encode::IsNull::No
    }
}

impl Position {
    #[inline(always)]
    pub fn new(x: i32, y: i32, map: MapPosition) -> Position {
        Position { x, y, map }
    }

    pub fn left_map(&self) -> bool {
        self.x < 0 || self.x >= MAP_MAX_X as i32 || self.y < 0 || self.y >= MAP_MAX_Y as i32
    }

    pub fn checkdistance(&self, target: Position) -> i32 {
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

    pub fn map_offset(&self, dir: MapPosDir) -> Position {
        match dir {
            MapPosDir::UpLeft => Position::new(
                self.x - MAP_MAX_X as i32,
                self.y - MAP_MAX_Y as i32,
                self.map,
            ),
            MapPosDir::Up => Position::new(self.x, self.y - MAP_MAX_Y as i32, self.map),
            MapPosDir::UpRight => Position::new(
                self.x + MAP_MAX_X as i32,
                self.y - MAP_MAX_Y as i32,
                self.map,
            ),
            MapPosDir::Left => Position::new(self.x - MAP_MAX_X as i32, self.y, self.map),
            MapPosDir::None | MapPosDir::Center => Position::new(self.x, self.y, self.map),
            MapPosDir::Right => Position::new(self.x + MAP_MAX_X as i32, self.y, self.map),
            MapPosDir::DownLeft => Position::new(
                self.x - MAP_MAX_X as i32,
                self.y + MAP_MAX_Y as i32,
                self.map,
            ),
            MapPosDir::Down => Position::new(self.x, self.y + MAP_MAX_Y as i32, self.map),
            MapPosDir::DownRight => Position::new(
                self.x + MAP_MAX_X as i32,
                self.y + MAP_MAX_Y as i32,
                self.map,
            ),
        }
    }

    pub fn update_pos_map(&mut self, storage: &Storage) -> bool {
        let set_pos = |pos: &mut Position, mappos, x, y| -> bool {
            let mapid = match get_dir_mapid(storage, pos.map, mappos) {
                Some(id) => id,
                None => return false,
            };

            *pos = Position::new(x, y, mapid);
            true
        };

        //precheck to make sure its not outside the 9 by 9 map area so calculations are correct.
        //TODO: Make this work for further outside the default map zones.
        if self.x > 63 || self.x < -63 || self.y > 63 || self.y < 63 {
            return false;
        }

        match (self.x, self.y) {
            (x, y) if x < 0 && y < 0 => set_pos(
                self,
                MapPosDir::UpLeft,
                MAP_MAX_X as i32 - x,
                MAP_MAX_Y as i32 - y,
            ),
            (x, y) if x >= MAP_MAX_X as i32 && y < 0 => set_pos(
                self,
                MapPosDir::UpRight,
                x - MAP_MAX_X as i32,
                MAP_MAX_Y as i32 - y,
            ),
            (x, y) if x < 0 && y >= MAP_MAX_Y as i32 => set_pos(
                self,
                MapPosDir::DownLeft,
                MAP_MAX_X as i32 - x,
                y - MAP_MAX_Y as i32,
            ),
            (x, y) if x >= MAP_MAX_X as i32 && y >= MAP_MAX_Y as i32 => set_pos(
                self,
                MapPosDir::DownRight,
                x - MAP_MAX_X as i32,
                y - MAP_MAX_Y as i32,
            ),
            (x, y) if x < 0 => set_pos(self, MapPosDir::Left, MAP_MAX_X as i32 - x, y),
            (x, y) if y >= MAP_MAX_Y as i32 => {
                set_pos(self, MapPosDir::Up, x, MAP_MAX_Y as i32 - y)
            }
            (x, y) if x >= MAP_MAX_X as i32 => {
                set_pos(self, MapPosDir::Right, x - MAP_MAX_X as i32, y)
            }
            (x, y) if y < 0 => set_pos(self, MapPosDir::Down, x, y - MAP_MAX_Y as i32),
            (_, _) => true,
        }
    }

    //must be gaurenteed to fit within the Grid. Or errors will occur.
    #[inline]
    pub fn as_tile(&self) -> usize {
        ((self.y * (MAP_MAX_X as i32 - 1)) + self.x) as usize
    }
}

#[inline]
pub fn in_range(range: i32, target: Position, attacker: Position) -> bool {
    attacker.checkdistance(target) <= range
}
