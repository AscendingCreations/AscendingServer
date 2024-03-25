use crate::{containers::*, gametypes::*, players::*, sql::*};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use hecs::World;
use sqlx::{FromRow, PgPool};
use tokio::{runtime::Runtime, task};

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct Check {
    pub check: bool,
}

pub fn initiate(conn: &PgPool, rt: &mut Runtime, local: &task::LocalSet) -> Result<()> {
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

pub fn find_player(storage: &Storage, email: &str, password: &str) -> Result<Option<i64>> {
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

pub fn check_existance(storage: &Storage, username: &str, email: &str) -> Result<i64> {
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

pub fn new_player(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    username: String,
    email: String,
    password: String,
) -> Result<i64> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let player = PGPlayer::new(world, entity, username, email, password);
    let uid: (i64, ) = local.block_on(&rt,
        sqlx::query_as(r#"
        INSERT INTO public.player(
            username, address, password, itemtimer, deathtimer, vals, spawn, pos, email, sprite, indeath, level, levelexp, resetcount, pk, data, vital, access)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) RETURNING uid;
        "#)
            .bind(player.username)
            .bind(player.address)
            .bind(player.password)
            .bind(player.itemtimer)
            .bind(player.deathtimer)
            .bind(player.vals)
            .bind(player.spawn)
            .bind(player.pos)
            .bind(player.email)
            .bind(player.sprite)
            .bind(player.indeath)
            .bind(player.level)
            .bind(player.levelexp)
            .bind(player.resetcount)
            .bind(player.pk)
            .bind(player.data)
            .bind(player.vital)
            .bind(player.access)
            .fetch_one(&storage.pgconn),
    )?;

    let inv_insert = PGInvItem::into_insert_all(PGInvItem::new(
        &world.cloned_get_or_panic::<Inventory>(entity).items,
        uid.0,
    ));

    local.block_on(&rt, async {
        for script in inv_insert {
            match sqlx::query(&script).execute(&storage.pgconn).await {
                Ok(_) => continue,
                Err(e) => return Err(e),
            };
        }

        Ok(())
    })?;

    let storage_insert = PGStorageItem::into_insert_all(PGStorageItem::new(
        &world.cloned_get_or_panic::<PlayerStorage>(entity).items,
        uid.0,
    ));

    local.block_on(&rt, async {
        for script in storage_insert {
            match sqlx::query(&script).execute(&storage.pgconn).await {
                Ok(_) => continue,
                Err(e) => return Err(e),
            };
        }

        Ok(())
    })?;

    let equip_insert = PGEquipItem::into_insert_all(PGEquipItem::new(
        &world.cloned_get_or_panic::<Equipment>(entity).items,
        uid.0,
    ));

    local.block_on(&rt, async {
        for script in equip_insert {
            match sqlx::query(&script).execute(&storage.pgconn).await {
                Ok(_) => continue,
                Err(e) => return Err(e),
            };
        }

        Ok(())
    })?;
    Ok(uid.0)
}

pub fn load_player(storage: &Storage, world: &mut World, entity: &crate::Entity) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let accountid = world.get::<&Account>(entity.0).unwrap().id;

    let player_with_id: PGPlayerWithID = local.block_on(&rt,
        sqlx::query_as(r#"
        SELECT uid, username, address, password, itemtimer, deathtimer, vals, spawn, pos, email, sprite, indeath, level, levelexp, resetcount, pk, data, vital, passresetcode, access
	    FROM public.player where uid = $1;
        "#)
            .bind(accountid)
            .fetch_one(&storage.pgconn),
    )?;

    player_with_id.into_player(world, entity);

    let player_inv: Vec<PGInvItem> = local.block_on(
        &rt,
        sqlx::query_as(
            r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.inventory where uid = $1;
        "#,
        )
        .bind(accountid)
        .fetch_all(&storage.pgconn),
    )?;

    PGInvItem::array_into_items(
        player_inv,
        &mut world.get::<&mut Inventory>(entity.0)?.items,
    );

    let player_storage: Vec<PGStorageItem> = local.block_on(
        &rt,
        sqlx::query_as(
            r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.storage where uid = $1;
        "#,
        )
        .bind(accountid)
        .fetch_all(&storage.pgconn),
    )?;

    PGStorageItem::array_into_items(
        player_storage,
        &mut world.get::<&mut PlayerStorage>(entity.0)?.items,
    );

    let player_eqpt: Vec<PGEquipItem> = local.block_on(
        &rt,
        sqlx::query_as(
            r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.equipment where uid = $1;
        "#,
        )
        .bind(accountid)
        .fetch_all(&storage.pgconn),
    )?;

    PGEquipItem::array_into_items(
        player_eqpt,
        &mut world.get::<&mut Equipment>(entity.0)?.items,
    );

    Ok(())
}

pub fn update_player(storage: &Storage, world: &mut World, entity: &crate::Entity) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerLogOut::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
        UPDATE public.player
        SET itemtimer=$2, deathtimer=$3, pos=$4, vital=$5, indeath=$6, pk=$7
        WHERE uid = $1;
    "#,
        )
        .bind(player.uid)
        .bind(player.itemtimer)
        .bind(player.deathtimer)
        .bind(player.pos)
        .bind(player.vital)
        .bind(player.indeath)
        .bind(player.pk)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_inv(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let inv = world.cloned_get_or_panic::<Inventory>(entity);
    let account = world.cloned_get_or_panic::<Account>(entity);
    let update = PGInvItem::single(&inv.items, account.id, slot).into_update();

    local.block_on(&rt, sqlx::query(&update).execute(&storage.pgconn))?;
    Ok(())
}

pub fn update_storage(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player_storage = world.cloned_get_or_panic::<PlayerStorage>(entity);
    let account = world.cloned_get_or_panic::<Account>(entity);
    let update = PGStorageItem::single(&player_storage.items, account.id, slot).into_update();

    local.block_on(&rt, sqlx::query(&update).execute(&storage.pgconn))?;
    Ok(())
}

pub fn update_equipment(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let equip = world.cloned_get_or_panic::<Equipment>(entity);
    let account = world.cloned_get_or_panic::<Account>(entity);
    let update = PGEquipItem::single(&equip.items, account.id, slot).into_update();

    local.block_on(&rt, sqlx::query(&update).execute(&storage.pgconn))?;

    Ok(())
}

pub fn update_address(storage: &Storage, world: &mut World, entity: &crate::Entity) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerAddress::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET address=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.address)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_playerdata(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerData::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET data=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.data)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_passreset(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
    resetpassword: Option<String>,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerPassReset::new(world, entity, resetpassword);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET passresetcode=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.passresetcode)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_spawn(storage: &Storage, world: &mut World, entity: &crate::Entity) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerSpawn::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET spawn=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.spawn)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_pos(storage: &Storage, world: &mut World, entity: &crate::Entity) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerPos::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET pos=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.pos)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_currency(storage: &Storage, world: &mut World, entity: &crate::Entity) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerCurrency::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET vals=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.vals)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_level(storage: &Storage, world: &mut World, entity: &crate::Entity) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerLevel::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET level=$2, levelexp=$3, vital=$4
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.level)
        .bind(player.levelexp)
        .bind(player.vital)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_resetcount(
    storage: &Storage,
    world: &mut World,
    entity: &crate::Entity,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let player = PGPlayerReset::new(world, entity);

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET resetcount=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.resetcount)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}
