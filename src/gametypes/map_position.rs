use crate::gametypes::*;
use bytey::{ByteBufferRead, ByteBufferWrite};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    pg::{sql_types::Record, Pg},
    serialize::{self, Output, ToSql, WriteTuple},
    sql_types::{BigInt, Integer},
};
use serde::{Deserialize, Serialize};

#[derive(SqlType)]
#[diesel(postgres_type(name = "map_position"))]
pub struct MapPosType;

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Deserialize,
    Serialize,
    FromSqlRow,
    AsExpression,
    Hash,
    ByteBufferRead,
    ByteBufferWrite,
)]
#[diesel(sql_type = PosType)]
pub struct MapPosition {
    pub x: i32,
    pub y: i32,
    pub group: u64,
}

impl MapPosition {
    #[inline(always)]
    pub fn new(x: i32, y: i32, group: u64) -> MapPosition {
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

impl ToSql<MapPosType, Pg> for MapPosition {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        WriteTuple::<(Integer, Integer, BigInt)>::write_tuple(
            &(self.x, self.y, self.group as i64),
            out,
        )
    }
}

impl<DB> FromSql<MapPosType, DB> for MapPosition
where
    DB: Backend,
    (i32, i32, i64): FromSql<Record<(Integer, Integer, BigInt)>, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let data: (i32, i32, i64) =
            FromSql::<Record<(Integer, Integer, BigInt)>, DB>::from_sql(bytes)?;

        Ok(MapPosition {
            x: data.0,
            y: data.1,
            group: data.2 as u64,
        })
    }
}
