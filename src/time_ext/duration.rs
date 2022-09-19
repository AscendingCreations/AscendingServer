use byteorder::{NetworkEndian, WriteBytesExt};
use bytey::{ByteBuffer, ByteBufferRead, ByteBufferWrite};
use diesel::{
    deserialize::{self, FromSql},
    pg::Pg,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::BigInt,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, FromSqlRow, AsExpression, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[diesel(sql_type = BigInt)]
pub struct MyDuration(pub chrono::Duration);

impl MyDuration {
    pub fn milliseconds(mills: i64) -> MyDuration {
        MyDuration(chrono::Duration::milliseconds(mills))
    }

    pub fn as_std(&self) -> std::time::Duration {
        if let Ok(dur) = self.0.to_std() {
            dur
        } else {
            std::time::Duration::from_millis(0)
        }
    }
}

impl From<chrono::Duration> for MyDuration {
    fn from(duration: chrono::Duration) -> MyDuration {
        MyDuration(duration)
    }
}

impl AsRef<chrono::Duration> for MyDuration {
    fn as_ref(&self) -> &chrono::Duration {
        &self.0
    }
}

impl std::ops::Deref for MyDuration {
    type Target = chrono::Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToSql<BigInt, Pg> for MyDuration {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        out.write_i64::<NetworkEndian>(self.num_milliseconds())
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl FromSql<BigInt, Pg> for MyDuration {
    fn from_sql(value: diesel::backend::RawValue<'_, Pg>) -> deserialize::Result<Self> {
        let i64_value = FromSql::<BigInt, Pg>::from_sql(value)?;
        Ok(MyDuration(chrono::Duration::milliseconds(i64_value)))
    }
}

impl Serialize for MyDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.num_milliseconds().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MyDuration {
    fn deserialize<D>(deserializer: D) -> Result<MyDuration, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(MyDuration(chrono::Duration::milliseconds(
            i64::deserialize(deserializer)?,
        )))
    }
}

impl ByteBufferRead for MyDuration {
    fn read_from_buffer(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(MyDuration(chrono::Duration::milliseconds(
            buffer.read::<i64>()?,
        )))
    }

    fn read_from_buffer_le(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(MyDuration(chrono::Duration::milliseconds(
            buffer.read_le::<i64>()?,
        )))
    }

    fn read_from_buffer_be(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(MyDuration(chrono::Duration::milliseconds(
            buffer.read_be::<i64>()?,
        )))
    }
}

impl ByteBufferWrite for &MyDuration {
    fn write_to_buffer(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write(self.num_milliseconds())?;
        Ok(())
    }
    fn write_to_buffer_le(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_le(self.num_milliseconds())?;
        Ok(())
    }
    fn write_to_buffer_be(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_be(self.num_milliseconds())?;
        Ok(())
    }
}