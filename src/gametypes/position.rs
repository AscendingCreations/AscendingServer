use crate::gametypes::*;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
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
    Readable,
    Writable,
    MByteBufferRead,
    MByteBufferWrite,
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
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> std::result::Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder
            .encode(self.x)?
            .encode(self.y)?
            .encode(self.map)?
            .finish();

        Ok(sqlx::encode::IsNull::No)
    }
}

impl Position {
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

    pub fn checkdirection(&self, target: Position) -> Option<i32> {
        let dx = self.x - target.x;
        let dy = self.y - target.y;

        let abs_dx = dx.abs();
        let abs_dy = dy.abs();

        // 0 down, 1 right, 2 up, 3 left
        match (abs_dx > abs_dy, abs_dy > abs_dx) {
            (true, _) => match dx {
                x if x > 0 => Some(3),
                x if x < 0 => Some(1),
                _ => None,
            },
            (_, true) => match dy {
                y if y > 0 => Some(0),
                y if y < 0 => Some(2),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn map_offset(&self, dir: MapPosDir) -> Position {
        match dir {
            MapPosDir::UpLeft => Position::new(
                self.x - MAP_MAX_X as i32,
                self.y + MAP_MAX_Y as i32,
                self.map,
            ),
            MapPosDir::Up => Position::new(self.x, self.y + MAP_MAX_Y as i32, self.map),
            MapPosDir::UpRight => Position::new(
                self.x + MAP_MAX_X as i32,
                self.y + MAP_MAX_Y as i32,
                self.map,
            ),
            MapPosDir::Left => Position::new(self.x - MAP_MAX_X as i32, self.y, self.map),
            MapPosDir::None | MapPosDir::Center => Position::new(self.x, self.y, self.map),
            MapPosDir::Right => Position::new(self.x + MAP_MAX_X as i32, self.y, self.map),
            MapPosDir::DownLeft => Position::new(
                self.x - MAP_MAX_X as i32,
                self.y - MAP_MAX_Y as i32,
                self.map,
            ),
            MapPosDir::Down => Position::new(self.x, self.y - MAP_MAX_Y as i32, self.map),
            MapPosDir::DownRight => Position::new(
                self.x + MAP_MAX_X as i32,
                self.y - MAP_MAX_Y as i32,
                self.map,
            ),
        }
    }

    //must be gaurenteed to fit within the Grid. Or errors will occur.
    #[inline]
    pub fn as_tile(&self) -> usize {
        ((self.y * (MAP_MAX_X as i32)) + self.x) as usize
    }
}

#[inline]
pub fn in_range(range: i32, target: Position, attacker: Position) -> bool {
    attacker.checkdistance(target) <= range
}
