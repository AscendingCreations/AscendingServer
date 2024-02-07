use byteorder::{NetworkEndian, WriteBytesExt};
use chrono::{offset::Utc, Duration, NaiveDate};
use diesel::{
    deserialize::{self, FromSql},
    pg::{data_types::*, Pg},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::{self, Date},
};
use serde::{Deserialize, Serialize};
use unwrap_helpers::*;

#[derive(
    Debug,
    FromSqlRow,
    AsExpression,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[diesel(sql_type = Date)]
pub struct MyDate(pub chrono::NaiveDate);

impl MyDate {
    pub fn now() -> MyDate {
        MyDate(Utc::now().date_naive())
    }

    pub fn add_days(&mut self, days: i64) {
        self.0 = unwrap_or_return!(self.0.checked_add_signed(Duration::days(days)));
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

impl ToSql<sql_types::Date, Pg> for MyDate {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let days_since_epoch: i64 = self
            .0
            .signed_duration_since(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or_default())
            .num_days();

        out.write_i32::<NetworkEndian>(days_since_epoch as i32)
            .map(|_| IsNull::No)
            .map_err(|e| Box::new(e) as Box<_>)
    }
}

impl FromSql<sql_types::Date, Pg> for MyDate {
    fn from_sql(bytes: diesel::backend::RawValue<'_, Pg>) -> deserialize::Result<Self> {
        let PgDate(offset) = FromSql::<sql_types::Date, Pg>::from_sql(bytes)?;
        match NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap_or_default()
            .checked_add_signed(Duration::days(i64::from(offset)))
        {
            Some(date) => Ok(MyDate(date)),
            None => {
                let error_message =
                    format!("Chrono can only represent dates up to {:?}", NaiveDate::MAX);
                Err(error_message.into())
            }
        }
    }
}
