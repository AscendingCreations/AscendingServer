use crate::gametypes::*;
use bytey::{ByteBufferRead, ByteBufferWrite};
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
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
    MByteBufferRead,
    MByteBufferWrite,
)]
pub struct MapPosition {
    pub x: i32,
    pub y: i32,
    pub group: i32,
}

impl sqlx::Type<Postgres> for MapPosition {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("map_position")
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        *ty == Self::type_info()
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for MapPosition {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> sqlx::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let x = decoder.try_decode::<i32>()?;
        let y = decoder.try_decode::<i32>()?;
        let group = decoder.try_decode::<i32>()?;
        Ok(Self { x, y, group })
    }
}

impl<'q> sqlx::Encode<'q, Postgres> for MapPosition {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
        encoder
            .encode(self.x)
            .encode(self.y)
            .encode(self.group)
            .finish();

        sqlx::encode::IsNull::No
    }
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
            MapPosDir::UpLeft => MapPosition::new(self.x - 1, self.y + 1, self.group),
            MapPosDir::Up => MapPosition::new(self.x, self.y + 1, self.group),
            MapPosDir::UpRight => MapPosition::new(self.x + 1, self.y + 1, self.group),
            MapPosDir::Left => MapPosition::new(self.x - 1, self.y, self.group),
            MapPosDir::None | MapPosDir::Center => MapPosition::new(self.x, self.y, self.group),
            MapPosDir::Right => MapPosition::new(self.x + 1, self.y, self.group),
            MapPosDir::DownLeft => MapPosition::new(self.x - 1, self.y - 1, self.group),
            MapPosDir::Down => MapPosition::new(self.x, self.y - 1, self.group),
            MapPosDir::DownRight => MapPosition::new(self.x + 1, self.y - 1, self.group),
        }
    }
}
