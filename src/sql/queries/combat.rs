use crate::{containers::Storage, sql::integers::Shifting};
use itertools::Itertools;
use uuid::Uuid;

use crate::gametypes::*;

use sqlx::FromRow;

#[derive(Debug, FromRow, Default)]
pub struct PGCombat {
    pub indeath: bool,
    pub level: i32,
    pub levelexp: i64,
    pub pk: bool,
    pub vital: [i32; VITALS_MAX],
    pub vital_max: [i32; VITALS_MAX],
}

impl PGCombat {
    pub fn into_empty(uid: Uuid) -> String {
        format!(
            r#"
            INSERT INTO public.combat(uid, indeath, level, levelexp, pk, vital, vital_max)
            VALUES ('{0}', false, 0, {1}, false, '{{25, 2, 100}}', '{{25, 2, 100}}');
            "#,
            uid,
            i64::unshift_signed(&0),
        )
    }
}

pub fn sql_new_combat(storage: &Storage, uid: Uuid) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = PGCombat::into_empty(uid);
    local.block_on(&rt, sqlx::query(&query).execute(&storage.pgconn))?;

    Ok(())
}

pub fn sql_load_combat(storage: &Storage, account_id: Uuid) -> Result<PGCombat> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = format!(
        r#"
        SELECT indeath, level, levelexp, pk, vital, vital_max
        FROM public.combat
        WHERE uid = '{}';
        "#,
        account_id,
    );
    let data: PGCombat = local.block_on(&rt, sqlx::query_as(&query).fetch_one(&storage.pgconn))?;

    Ok(data)
}

pub fn sql_update_combat(storage: &Storage, uid: Uuid, data: PGCombat) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let vital = data
        .vital
        .iter()
        .format_with(", ", |elt, f| f(&format_args!("{}", elt)))
        .to_string();
    let vitalmax = data
        .vital_max
        .iter()
        .format_with(", ", |elt, f| f(&format_args!("{}", elt)))
        .to_string();

    let query_text = format!(
        r#"
        UPDATE public.combat
        SET indeath = {1},
            level = {2},
            levelexp = {3},
            pk = {4},
            vital = '{{{5}}}',
            vital_max = '{{{6}}}'
        WHERE uid = '{0}';
        "#,
        uid, data.indeath, data.level, data.levelexp, data.pk, vital, vitalmax
    );

    local.block_on(&rt, sqlx::query(&query_text).execute(&storage.pgconn))?;

    Ok(())
}

pub fn sql_update_level(storage: &Storage, uid: Uuid, data: PGCombat) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let vital = data
        .vital
        .iter()
        .format_with(", ", |elt, f| f(&format_args!("{}", elt)))
        .to_string();
    let vitalmax = data
        .vital_max
        .iter()
        .format_with(", ", |elt, f| f(&format_args!("{}", elt)))
        .to_string();

    let query_text = format!(
        r#"
        UPDATE public.combat
        SET level = {1},
            levelexp = {2},
            vital = '{{{3}}}',
            vital_max = '{{{4}}}'
        WHERE uid = '{0}';
        "#,
        uid, data.level, data.levelexp, vital, vitalmax
    );

    local.block_on(&rt, sqlx::query(&query_text).execute(&storage.pgconn))?;

    Ok(())
}
