use crate::{containers::*, gametypes::*, sql::*};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use chrono::Duration;
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

pub use account::*;
pub use combat::*;
pub use equipment::*;
pub use general::*;
pub use inventory::*;
pub use location::*;
pub use storage::*;

use super::integers::Shifting;

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
        LOGTYPE_SCHEMA,
        LOGTYPE_SCHEMA_ALTER,
        USERACCESS_SCHEMA,
        USERACCESS_SCHEMA_ALTER,
        MAP_POSITION_SCHEMA,
        MAP_POSITION_SCHEMA_ALTER,
        POSITION_SCHEMA,
        POSITION_SCHEMA_ALTER,
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
    let tick = *storage.gettick.borrow();

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
        p_data.sprite.id = general_data.sprite.shift_signed();
        p_data.money.vals = general_data.money.shift_signed();
        p_data.general.resetcount = general_data.resetcount;
        p_data.item_timer.itemtimer =
            tick + Duration::try_milliseconds(general_data.itemtimer).unwrap_or_default();
        p_data.combat.death_timer.0 =
            tick + Duration::try_milliseconds(general_data.deathtimer).unwrap_or_default();

        let equipment_data = sql_load_equipment(storage, account_id)?;
        for eq_data in equipment_data.slot.iter() {
            if let Some(data) = p_data.equipment.items.get_mut(eq_data.id as usize) {
                data.num = eq_data.num.shift_signed();
                data.val = eq_data.val.shift_signed();
                data.level = eq_data.level as u8;
                data.data = eq_data.data;
            }
        }

        let inventory_data = sql_load_inventory(storage, account_id)?;
        for inv_data in inventory_data.slot.iter() {
            if let Some(data) = p_data.inventory.items.get_mut(inv_data.id as usize) {
                data.num = inv_data.num.shift_signed();
                data.val = inv_data.val.shift_signed();
                data.level = inv_data.level as u8;
                data.data = inv_data.data;
            }
        }

        let storage_data = sql_load_storage(storage, account_id)?;
        for item_data in storage_data.slot.iter() {
            if let Some(data) = p_data.storage.items.get_mut(item_data.id as usize) {
                data.num = item_data.num.shift_signed();
                data.val = item_data.val.shift_signed();
                data.level = item_data.level as u8;
                data.data = item_data.data;
            }
        }

        let combat_data = sql_load_combat(storage, account_id)?;
        p_data.general.pk = combat_data.pk;
        p_data.general.levelexp = combat_data.levelexp.shift_signed();
        p_data.combat.level = combat_data.level;
        p_data.combat.vitals.vital = combat_data.vital;
        p_data.combat.vitals.vitalmax = combat_data.vital_max;

        let location_data = sql_load_location(storage, account_id)?;
        p_data.movement.pos = location_data.pos;
        p_data.movement.spawn.pos = location_data.spawn;
        p_data.movement.dir = location_data.dir as u8;
    }
    Ok(())
}
