use byteorder::{NetworkEndian, WriteBytesExt};
use bytey::{ByteBuffer, ByteBufferRead, ByteBufferWrite};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::BigInt,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::{Postgres, Type};
use std::{ops::Add, time::Instant};

#[derive(Debug, FromSqlRow, AsExpression, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[diesel(sql_type = BigInt)]
pub struct MyInstant(pub std::time::Instant);

impl MyInstant {
    pub fn now() -> MyInstant {
        MyInstant(Instant::now())
    }

    pub fn to_dur(self) -> i64 {
        let mut dur: i64 = 0;

        if let Ok(approx) =
            chrono::Duration::from_std(self.0.saturating_duration_since(Instant::now()))
        {
            if approx > chrono::Duration::milliseconds(1) {
                dur = approx.num_milliseconds();
            }
        }

        dur
    }

    pub fn from_dur(dur: i64) -> MyInstant {
        let duration = chrono::Duration::milliseconds(dur);
        let mut instant_now = Instant::now();

        if let Ok(dur) = duration.to_std() {
            instant_now += dur;
        }

        MyInstant(instant_now)
    }
}

impl From<Instant> for MyInstant {
    fn from(instant: Instant) -> MyInstant {
        MyInstant(instant)
    }
}

impl AsRef<Instant> for MyInstant {
    fn as_ref(&self) -> &Instant {
        &self.0
    }
}

impl std::ops::Deref for MyInstant {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToSql<BigInt, Pg> for MyInstant {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        out.write_i64::<NetworkEndian>(self.to_dur())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl<DB> FromSql<BigInt, DB> for MyInstant
where
    DB: Backend,
    i64: FromSql<BigInt, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let i64_value = FromSql::<BigInt, DB>::from_sql(bytes)?;
        Ok(MyInstant::from_dur(i64_value))
    }
}

impl sqlx::Type<Postgres> for MyInstant {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <i64 as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
        *ty == Self::type_info()
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for MyInstant {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> sqlx::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let value = decoder.try_decode::<i64>()?;
        Ok(Self::from_dur(value))
    }
}

impl<'q> sqlx::Encode<'q, Postgres> for MyInstant {
    fn encode_by_ref(&self, buf: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
        <i64 as sqlx::Encode<Postgres>>::encode(self.to_dur(), buf)
    }
}

impl Serialize for MyInstant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_dur().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MyInstant {
    fn deserialize<D>(deserializer: D) -> Result<MyInstant, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(MyInstant::from_dur(i64::deserialize(deserializer)?))
    }
}

impl ByteBufferRead for MyInstant {
    fn read_from_buffer(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(MyInstant::from_dur(buffer.read::<i64>()?))
    }

    fn read_from_buffer_le(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(MyInstant::from_dur(buffer.read_le::<i64>()?))
    }

    fn read_from_buffer_be(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(MyInstant::from_dur(buffer.read_be::<i64>()?))
    }
}

impl ByteBufferWrite for &MyInstant {
    fn write_to_buffer(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write(self.to_dur())?;
        Ok(())
    }
    fn write_to_buffer_le(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_le(self.to_dur())?;
        Ok(())
    }
    fn write_to_buffer_be(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_be(self.to_dur())?;
        Ok(())
    }
}

impl Add<chrono::Duration> for MyInstant {
    type Output = MyInstant;

    fn add(self, other: chrono::Duration) -> MyInstant {
        if let Ok(dur) = other.to_std() {
            MyInstant(self.0 + dur)
        } else {
            MyInstant(self.0)
        }
    }
}

impl Add<std::time::Duration> for MyInstant {
    type Output = MyInstant;

    fn add(self, other: std::time::Duration) -> MyInstant {
        MyInstant(self.0 + other)
    }
}
