use uuid::Uuid;

use crate::{containers::Storage, gametypes::*};

use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct PGLocation {
    pub spawn: Position,
    pub pos: Position,
    pub dir: i16,
}

impl PGLocation {
    pub fn into_empty(uid: Uuid) -> String {
        format!(
            r#"
            INSERT INTO public.locations(uid, spawn, pos, dir)
            VALUES ('{0}', $1, $2, 0);
            "#,
            uid,
        )
    }
}

pub fn sql_new_location(storage: &Storage, uid: Uuid) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = PGLocation::into_empty(uid);
    local.block_on(
        &rt,
        sqlx::query(&query)
            .bind(Position::default())
            .bind(Position::default())
            .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn sql_load_location(storage: &Storage, account_id: Uuid) -> Result<PGLocation> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = format!(
        r#"
        SELECT spawn, pos, dir
        FROM public.locations
        WHERE uid = '{}';
        "#,
        account_id,
    );
    let data: PGLocation =
        local.block_on(&rt, sqlx::query_as(&query).fetch_one(&storage.pgconn))?;

    Ok(data)
}

pub fn sql_update_location(storage: &Storage, uid: Uuid, data: PGLocation) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query_text = format!(
        r#"
        UPDATE public.locations
        SET spawn = $1, pos = $2, dir = {1}
        WHERE uid = '{0}';
        "#,
        uid, data.dir
    );

    local.block_on(
        &rt,
        sqlx::query(&query_text)
            .bind(data.spawn)
            .bind(data.pos)
            .execute(&storage.pgconn),
    )?;

    Ok(())
}
