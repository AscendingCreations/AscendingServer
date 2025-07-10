use crate::{containers::Storage, sql::integers::Shifting};
use uuid::Uuid;

use crate::gametypes::*;

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct PGGeneral {
    pub sprite: i16,
    pub money: i64,
    pub resetcount: i16,
    pub itemtimer: i64,
    pub deathtimer: i64,
}

impl PGGeneral {
    pub fn into_empty(uid: Uuid) -> String {
        format!(
            r#"
            INSERT INTO public.general(uid, sprite, money, resetcount, itemtimer, deathtimer)
            VALUES ('{0}', {1}, {2}, 0, 0, 0);
            "#,
            uid,
            i16::unshift_signed(&0),
            i64::unshift_signed(&0),
        )
    }
}

pub fn sql_new_general(storage: &Storage, uid: Uuid) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = PGGeneral::into_empty(uid);
    local.block_on(&rt, sqlx::query(&query).execute(&storage.pgconn))?;

    Ok(())
}

pub fn sql_load_general(storage: &Storage, account_id: Uuid) -> Result<PGGeneral> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = format!(
        r#"
        SELECT sprite, money, resetcount, itemtimer, deathtimer
        FROM public.general
        WHERE uid = '{account_id}';
        "#,
    );
    let data: PGGeneral = local.block_on(&rt, sqlx::query_as(&query).fetch_one(&storage.pgconn))?;

    Ok(data)
}

pub fn sql_update_general(storage: &Storage, uid: Uuid, data: PGGeneral) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query_text = format!(
        r#"
        UPDATE public.general
        SET sprite = {1},
            money = {2},
            resetcount = {3},
            itemtimer = {4},
            deathtimer = {5}
        WHERE uid = '{0}';
        "#,
        uid, data.sprite, data.money, data.resetcount, data.itemtimer, data.deathtimer,
    );

    local.block_on(&rt, sqlx::query(&query_text).execute(&storage.pgconn))?;

    Ok(())
}

pub fn sql_update_resetcount(storage: &Storage, uid: Uuid, resetcount: i16) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query_text = format!(
        r#"
        UPDATE public.general
        SET resetcount = {resetcount}
        WHERE uid = '{uid}';
        "#
    );

    local.block_on(&rt, sqlx::query(&query_text).execute(&storage.pgconn))?;

    Ok(())
}

pub fn sql_update_money(storage: &Storage, uid: Uuid, money: i64) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query_text = format!(
        r#"
        UPDATE public.general
        SET money = {money}
        WHERE uid = '{uid}';
        "#,
    );

    local.block_on(&rt, sqlx::query(&query_text).execute(&storage.pgconn))?;

    Ok(())
}
