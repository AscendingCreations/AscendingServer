use crate::{containers::*, gametypes::*, sql::*};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::{FromRow, PgPool};
use tokio::{runtime::Runtime, task};
use uuid::Uuid;

mod account;
mod combat;
mod equipment;
mod general;
mod inventory;
mod location;
mod storage;

use account::*;
use combat::*;
use equipment::*;
use general::*;
use inventory::*;
use location::*;
use storage::*;

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct PlayerWithPassword {
    pub uid: Uuid,
    pub password: String,
}

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct Check {
    pub check: bool,
}

pub fn initiate(conn: &PgPool, rt: &mut Runtime, local: &task::LocalSet) -> Result<()> {
    let queries = [
        PG_CRYPTO_EXTENSION,
        PG_UUID,
        LOGS_SCHEMA,
        LOGS_SCHEMA_ALTER,
        ACCOUNT_SCHEMA,
        ACCOUNT_SCHEMA_ALTER,
        GENERAL_SCHEMA,
        GENERAL_SCHEMA_ALTER,
        LOCATION_SCHEMA,
        LOCATION_SCHEMA_ALTER,
        COMBAT_SCHEMA,
        COMBAT_SCHEMA_ALTER,
        EQUIPMENT_SCHEMA,
        EQUIPMENT_SCHEMA_ALTER,
        INVENTORY_SCHEMA,
        INVENTORY_SCHEMA_ALTER,
        STORAGE_SCHEMA,
        STORAGE_SCHEMA_ALTER,
    ];

    for quere in queries {
        local.block_on(rt, sqlx::query(quere).execute(conn))?;
    }

    Ok(())
}

pub fn find_player(storage: &Storage, email: &str, password: &str) -> Result<Option<Uuid>> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let userdata: Option<PlayerWithPassword> = local.block_on(
        &rt,
        sqlx::query_as(
            r#"
                SELECT uid, password FROM public.account
                WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&storage.pgconn),
    )?;

    if let Some(userdata) = userdata {
        let hash = match PasswordHash::new(&userdata.password[..]) {
            Ok(v) => v,
            Err(_) => return Err(AscendingError::IncorrectPassword),
        };

        if Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
        {
            Ok(Some(userdata.uid))
        } else {
            Err(AscendingError::IncorrectPassword)
        }
    } else {
        Ok(None)
    }
}

pub fn check_existance(storage: &Storage, username: &str, email: &str) -> Result<i64> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let check: Check = local.block_on(
        &rt,
        sqlx::query_as(r#"SELECT EXISTS(SELECT 1 FROM public.account WHERE username=$1) as check"#)
            .bind(username)
            .fetch_one(&storage.pgconn),
    )?;

    if check.check {
        return Ok(1);
    };

    let check: Check = local.block_on(
        &rt,
        sqlx::query_as(r#"SELECT EXISTS(SELECT 1 FROM public.account WHERE email=$1) as check"#)
            .bind(email)
            .fetch_one(&storage.pgconn),
    )?;

    if check.check {
        return Ok(2);
    };

    Ok(0)
}

pub fn new_player(
    storage: &Storage,
    username: String,
    email: String,
    password: String,
    socket: &Socket,
) -> Result<Uuid> {
    let uid: Uuid = sql_new_account(storage, &username, &socket.addr, &password, &email)?;

    sql_new_general(storage, uid)?;
    sql_new_equipment(storage, uid)?;
    sql_new_inventory(storage, uid)?;
    sql_new_storage(storage, uid)?;
    sql_new_combat(storage, uid)?;
    sql_new_location(storage, uid)?;

    Ok(uid)
}

pub fn load_player(
    storage: &Storage,
    world: &mut World,
    entity: GlobalKey,
    account_id: Uuid,
) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        let account_data = sql_load_account(storage, account_id)?;
        p_data.user_access = account_data.useraccess;
        p_data.account.id = account_id;
        p_data.account.username.clone_from(&account_data.username);
        p_data
            .account
            .passresetcode
            .clone_from(&account_data.passresetcode);

        let general_data = sql_load_general(storage, account_id)?;

        let equipment_data = sql_load_equipment(storage, account_id)?;

        let inventory_data = sql_load_inventory(storage, account_id)?;

        let storage_data = sql_load_storage(storage, account_id)?;

        let combat_data = sql_load_combat(storage, account_id)?;

        let location_data = sql_load_location(storage, account_id)?;
    }
    Ok(())
}
