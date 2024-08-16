use std::backtrace::Backtrace;

use crate::{containers::*, gametypes::*, players::*, sql::integers::Shifting, sql::*};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use hecs::{NoSuchEntity, World};
use log::error;
use password_hash::SaltString;
use sqlx::{FromRow, PgPool};
use tokio::{runtime::Runtime, task};

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct Check {
    pub check: bool,
}

pub async fn initiate(conn: &PgPool, rt: &mut Runtime, local: &task::LocalSet) -> Result<()> {
    let queries = [
        LOGTYPE_SCHEMA,
        LOGTYPE_SCHEMA_ALTER,
        USERACCESS_SCHEMA,
        USERACCESS_SCHEMA_ALTER,
        MAP_POSITION_SCHEMA,
        MAP_POSITION_SCHEMA_ALTER,
        POSITION_SCHEMA,
        POSITION_SCHEMA_ALTER,
        PLAYER_SEQ_SCHEMA,
        PLAYER_SEQ_SCHEMA_ALTER,
        PLAYER_SCHEMA,
        PLAYER_SCHEMA_ALTER,
        EQUIPMENT_SCHEMA,
        EQUIPMENT_SCHEMA_ALTER,
        INVENTORY_SCHEMA,
        INVENTORY_SCHEMA_ALTER,
        STORAGE_SCHEMA,
        STORAGE_SCHEMA_ALTER,
        LOGS_SCHEMA,
        LOGS_SCHEMA_ALTER,
    ];

    for quere in queries {
        local.block_on(rt, sqlx::query(quere).execute(conn))?;
    }

    Ok(())
}

pub async fn find_player(storage: &Storage, email: &str, password: &str) -> Result<Option<i64>> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let userdata: Option<PlayerWithPassword> = local.block_on(
        &rt,
        sqlx::query_as(
            r#"
        SELECT uid, password FROM player
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

pub async fn check_existance(storage: &Storage, username: &str, email: &str) -> Result<i64> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let check: Check = local.block_on(
        &rt,
        sqlx::query_as(r#"SELECT EXISTS(SELECT 1 FROM player WHERE username=$1) as check"#)
            .bind(username)
            .fetch_one(&storage.pgconn),
    )?;

    if check.check {
        return Ok(1);
    };

    let check: Check = local.block_on(
        &rt,
        sqlx::query_as(r#"SELECT EXISTS(SELECT 1 FROM player WHERE email=$1) as check"#)
            .bind(email)
            .fetch_one(&storage.pgconn),
    )?;

    if check.check {
        return Ok(2);
    };

    Ok(0)
}

pub async fn new_player(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    username: String,
    email: String,
    password: String,
) -> Result<i64> {
    let argon = Argon2::default();
    let hashed_password = if let Ok(salt) = SaltString::encode_b64(SALT) {
        if let Ok(hash) = argon.hash_password(password.as_bytes(), &salt) {
            hash.to_string()
        } else {
            String::from("FailedPasswordHash")
        }
    } else {
        String::from("FailedPasswordHash")
    };

    let mut query = world.query_one::<PlayerQuery>(entity.0)?;
    let uid: (i64,) = if let Some((
        _account,
        socket,
        sprite,
        spawn,
        itemtimer,
        money,
        entity_data,
        access,
        position,
        vitals,
        death_timer,
        death_type,
        level,
        player,
    )) = query.get()
    {
        sqlx::query_as(r#"
        INSERT INTO public.player(
            username, address, password, itemtimer, deathtimer, vals, spawn, pos, email, sprite, indeath, level, levelexp, resetcount, pk, data, vital, vital_max, access)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19) RETURNING uid;
        "#)
            .bind(&username)
            .bind(&*socket.addr)
            .bind(hashed_password)
            .bind(itemtimer.itemtimer)
            .bind(death_timer.0)
            .bind(i64::unshift_signed(&money.vals))
            .bind(spawn.pos)
            .bind(position)
            .bind(&email)
            .bind(i16::unshift_signed(&sprite.id))
            .bind(death_type.is_spirit())
            .bind(level.0)
            .bind(i64::unshift_signed(&player.levelexp))
            .bind(player.resetcount)
            .bind(player.pk)
            .bind(entity_data.0)
            .bind(vitals.vital)
            .bind(vitals.vitalmax)
            .bind(access)
            .fetch_one(&storage.pgconn).await?
    } else {
        error!("missing components in new player");
        return Err(AscendingError::HecNoEntity {
            error: NoSuchEntity,
            backtrace: Box::new(Backtrace::capture()),
        });
    };

    let inv_insert = PGInvItem::into_insert_all(
        PGInvItem::new(&world.get::<&Inventory>(entity.0)?.items, uid.0).await,
    )
    .await;

    for script in inv_insert {
        sqlx::query(&script).execute(&storage.pgconn).await?;
    }

    let storage_insert = PGStorageItem::into_insert_all(
        PGStorageItem::new(&world.get::<&PlayerStorage>(entity.0)?.items, uid.0).await,
    )
    .await;

    for script in storage_insert {
        sqlx::query(&script).execute(&storage.pgconn).await?;
    }

    let equip_insert = PGEquipItem::into_insert_all(
        PGEquipItem::new(&world.get::<&Equipment>(entity.0)?.items, uid.0).await,
    )
    .await;

    for script in equip_insert {
        sqlx::query(&script).execute(&storage.pgconn).await?;
    }

    Ok(uid.0)
}

pub async fn load_player(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    accountid: i64,
) -> Result<()> {
    let player_with_id: PGPlayerWithID =
        sqlx::query_as(r#"
        SELECT uid, username, address, password, itemtimer, deathtimer, vals, spawn, pos, email, sprite, indeath, level, levelexp, resetcount, pk, data, vital, vital_max, passresetcode, access
	    FROM public.player where uid = $1;
        "#)
            .bind(accountid)
            .fetch_one(&storage.pgconn).await?;

    player_with_id.into_player(world, entity).await?;

    let player_inv: Vec<PGInvItem> = sqlx::query_as(
        r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.inventory where uid = $1;
        "#,
    )
    .bind(accountid)
    .fetch_all(&storage.pgconn)
    .await?;

    PGInvItem::array_into_items(
        player_inv,
        &mut world.get::<&mut Inventory>(entity.0)?.items,
    )
    .await;

    let player_storage: Vec<PGStorageItem> = sqlx::query_as(
        r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.storage where uid = $1;
        "#,
    )
    .bind(accountid)
    .fetch_all(&storage.pgconn)
    .await?;

    PGStorageItem::array_into_items(
        player_storage,
        &mut world.get::<&mut PlayerStorage>(entity.0)?.items,
    )
    .await;

    let player_eqpt: Vec<PGEquipItem> = sqlx::query_as(
        r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.equipment where uid = $1;
        "#,
    )
    .bind(accountid)
    .fetch_all(&storage.pgconn)
    .await?;

    PGEquipItem::array_into_items(
        player_eqpt,
        &mut world.get::<&mut Equipment>(entity.0)?.items,
    )
    .await;

    Ok(())
}

pub async fn update_player(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(
        &Account,
        &PlayerItemTimer,
        &Position,
        &Vitals,
        &DeathTimer,
        &DeathType,
        &Player,
    )>(entity.0)?;

    if let Some((account, itemtimer, position, vitals, death_timer, death_type, player)) =
        query.get()
    {
        sqlx::query(
            r#"
        UPDATE public.player
        SET itemtimer=$2, deathtimer=$3, pos=$4, vital=$5, indeath=$6, pk=$7, vital_max=$8
        WHERE uid = $1;
    "#,
        )
        .bind(account.id)
        .bind(itemtimer.itemtimer)
        .bind(death_timer.0)
        .bind(position)
        .bind(vitals.vital)
        .bind(death_type.is_spirit())
        .bind(player.pk)
        .bind(vitals.vitalmax)
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}

pub async fn update_inv(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let mut query = world.query_one::<(&Inventory, &Account)>(entity.0)?;

    if let Some((inv, account)) = query.get() {
        let update = PGInvItem::single(&inv.items, account.id, slot)
            .await
            .into_update()
            .await;

        sqlx::query(&update).execute(&storage.pgconn).await?;
    }

    Ok(())
}

pub async fn update_storage(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let mut query = world.query_one::<(&PlayerStorage, &Account)>(entity.0)?;

    if let Some((player_storage, account)) = query.get() {
        let update = PGStorageItem::single(&player_storage.items, account.id, slot)
            .await
            .into_update()
            .await;

        sqlx::query(&update).execute(&storage.pgconn).await?;
    }

    Ok(())
}

pub async fn update_equipment(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let mut query = world.query_one::<(&Equipment, &Account)>(entity.0)?;

    if let Some((equip, account)) = query.get() {
        let update = PGEquipItem::single(&equip.items, account.id, slot)
            .await
            .into_update()
            .await;

        sqlx::query(&update).execute(&storage.pgconn).await?;
    }

    Ok(())
}

pub async fn update_address(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(&Account, &Socket)>(entity.0)?;

    if let Some((account, socket)) = query.get() {
        sqlx::query(
            r#"
                UPDATE public.player
                SET address=$2
                WHERE uid = $1;
            "#,
        )
        .bind(account.id)
        .bind(&*socket.addr)
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}

pub async fn update_playerdata(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(&Account, &EntityData)>(entity.0)?;

    if let Some((account, entity_data)) = query.get() {
        sqlx::query(
            r#"
                UPDATE public.player
                SET data=$2
                WHERE uid = $1;
            "#,
        )
        .bind(account.id)
        .bind(entity_data.0)
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}

pub async fn update_passreset(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    resetpassword: Option<String>,
) -> Result<()> {
    sqlx::query(
        r#"
                UPDATE public.player
                SET passresetcode=$2
                WHERE uid = $1;
            "#,
    )
    .bind(world.get::<&Account>(entity.0)?.id)
    .bind(resetpassword)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_spawn(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(&Account, &Spawn)>(entity.0)?;

    if let Some((account, spawn)) = query.get() {
        sqlx::query(
            r#"
                UPDATE public.player
                SET spawn=$2
                WHERE uid = $1;
            "#,
        )
        .bind(account.id)
        .bind(spawn.pos)
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}

pub async fn update_pos(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(&Account, &Position)>(entity.0)?;

    if let Some((account, position)) = query.get() {
        sqlx::query(
            r#"
                UPDATE public.player
                SET pos=$2
                WHERE uid = $1;
            "#,
        )
        .bind(account.id)
        .bind(position)
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}

pub async fn update_currency(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(&Account, &Money)>(entity.0)?;

    if let Some((account, money)) = query.get() {
        sqlx::query(
            r#"
                UPDATE public.player
                SET vals=$2
                WHERE uid = $1;
            "#,
        )
        .bind(account.id)
        .bind(i64::unshift_signed(&money.vals))
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}

pub async fn update_level(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(&Account, &Level, &Player, &Vitals)>(entity.0)?;

    if let Some((account, level, player, vitals)) = query.get() {
        sqlx::query(
            r#"
                UPDATE public.player
                SET level=$2, levelexp=$3, vital=$4, vital_max=$5
                WHERE uid = $1;
            "#,
        )
        .bind(account.id)
        .bind(level.0)
        .bind(i64::unshift_signed(&player.levelexp))
        .bind(vitals.vital)
        .bind(vitals.vitalmax)
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}

pub async fn update_resetcount(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let mut query = world.query_one::<(&Account, &Player)>(entity.0)?;

    if let Some((account, player)) = query.get() {
        sqlx::query(
            r#"
                UPDATE public.player
                SET resetcount=$2
                WHERE uid = $1;
            "#,
        )
        .bind(account.id)
        .bind(player.resetcount)
        .execute(&storage.pgconn)
        .await?;
    }

    Ok(())
}
