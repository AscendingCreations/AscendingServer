use chrono::{offset::Utc, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Type};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MyDate(pub NaiveDate);

impl sqlx::Type<Postgres> for MyDate {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        <NaiveDate as Type<Postgres>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for MyDate {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> sqlx::Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let date = <NaiveDate as sqlx::Decode<Postgres>>::decode(value)?;
        Ok(Self(date))
    }
}

impl<'q> sqlx::Encode<'q, Postgres> for MyDate {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> std::result::Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <NaiveDate as sqlx::Encode<Postgres>>::encode(self.0, buf)
    }
}

impl MyDate {
    pub fn now() -> MyDate {
        MyDate(Utc::now().date_naive())
    }

    pub fn add_days(&mut self, days: i64) {
        if let Some(i) = self
            .0
            .checked_add_signed(Duration::try_days(days).unwrap_or_default())
        {
            self.0 = i;
        }
    }
}

impl From<chrono::NaiveDate> for MyDate {
    fn from(date: chrono::NaiveDate) -> MyDate {
        MyDate(date)
    }
}

impl AsRef<chrono::NaiveDate> for MyDate {
    fn as_ref(&self) -> &chrono::NaiveDate {
        &self.0
    }
}

impl std::ops::Deref for MyDate {
    type Target = chrono::NaiveDate;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
