use crate::{containers::Storage, sql::integers::Shifting};
use itertools::Itertools;
use uuid::Uuid;

use crate::gametypes::*;

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct PGStorageSlot {
    pub id: i16,
    pub num: i32,
    pub val: i16,
    pub level: i16,
    pub data: [i16; 5],
}

#[derive(Debug, FromRow)]
pub struct PGStorage {
    pub slot: Vec<PGStorageSlot>,
}

impl PGStorage {
    pub fn into_empty(uid: Uuid) -> String {
        let default_i32 = i32::unshift_signed(&0);
        let default_i16 = i16::unshift_signed(&0);

        let value_text = (0..MAX_INV)
            .map(|index| {
                format!(
                    "('{}', {}, {}, {}, 0, '{{0, 0, 0, 0, 0}}')",
                    uid, index, default_i32, default_i16
                )
            })
            .join(", ");

        format!(
            r#"
            INSERT INTO public.storage(uid, id, num, val, level, data)
            VALUES {0};
            "#,
            value_text
        )
    }
}

pub fn sql_new_storage(storage: &Storage, uid: Uuid) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = PGStorage::into_empty(uid);
    local.block_on(&rt, sqlx::query(&query).execute(&storage.pgconn))?;

    Ok(())
}

pub fn sql_load_storage(storage: &Storage, account_id: Uuid) -> Result<PGStorage> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = format!(
        r#"
        SELECT id, num, val, level, data
        FROM public.storage
        WHERE uid = '{}'
        ORDER BY id ASC;
        "#,
        account_id,
    );
    let data = PGStorage {
        slot: local.block_on(&rt, sqlx::query_as(&query).fetch_all(&storage.pgconn))?,
    };

    Ok(data)
}

pub fn sql_update_storage_slot(storage: &Storage, uid: Uuid, data: PGStorageSlot) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let data_str = data
        .data
        .iter()
        .format_with(", ", |elt, f| f(&format_args!("{}", elt)))
        .to_string();

    let query_text = format!(
        r#"
        UPDATE public.storage
        SET num = {2}, val = {3}, level = {4}, data = '{{{5}}}'
        WHERE uid = '{0}' AND id = {1};
        "#,
        uid, data.id, data.num, data.val, data.level, data_str
    );

    local.block_on(&rt, sqlx::query(&query_text).execute(&storage.pgconn))?;

    Ok(())
}
