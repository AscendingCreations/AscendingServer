use crate::{containers::*, gametypes::*, players::*, sql::*};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use futures::executor::block_on;
use sqlx::{FromRow, PgPool};

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct Check {
    pub check: bool,
}

pub fn find_player(conn: &PgPool, email: &str, password: &str) -> Result<Option<i64>> {
    let userdata: Option<PlayerWithPassword> = block_on(
        sqlx::query_as(
            r#"
        SELECT uid, password FROM player
        WHERE email = $1
    "#,
        )
        .bind(email)
        .fetch_optional(conn),
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

pub fn check_existance(conn: &PgPool, username: &str, email: &str) -> Result<i64> {
    let check: Check = block_on(
        sqlx::query_as(r#"SELECT EXISTS(SELECT 1 FROM player WHERE username=$1) as check"#)
            .bind(username)
            .fetch_one(conn),
    )?;

    if check.check {
        return Ok(1);
    };

    let check: Check = block_on(
        sqlx::query_as(r#"SELECT EXISTS(SELECT 1 FROM player WHERE email=$1) as check"#)
            .bind(email)
            .fetch_one(conn),
    )?;

    if check.check {
        return Ok(2);
    };

    Ok(0)
}

pub fn new_player(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
    username: String,
    email: String,
    password: String,
) -> Result<i64> {
    let player = PGPlayer::new(world, entity, username, email, password);
    let uid: (i64, ) = block_on(
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
            .bind(player.levelexp)
            .bind(player.resetcount)
            .bind(player.pk)
            .bind(player.data)
            .bind(player.vital)
            .bind(player.access)
            .fetch_one(conn),
    )?;

    let inv_insert = PGInvItem::into_insert_all(PGInvItem::new(
        &world.get_or_panic::<&Inventory>(entity).items,
        uid.0,
    ));

    block_on(sqlx::query(&inv_insert).execute(conn))?;

    let equip_insert = PGEquipItem::into_insert_all(PGEquipItem::new(
        &world.get_or_panic::<&Equipment>(entity).items,
        uid.0,
    ));

    block_on(sqlx::query(&equip_insert).execute(conn))?;
    Ok(uid.0)
}

pub fn load_player(
    _: &Storage,
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    let accountid = world.get_or_panic::<&Account>(entity).id;

    let player_with_id: PGPlayerWithID = block_on(
        sqlx::query_as(r#"
        SELECT uid, username, address, password, itemtimer, deathtimer, vals, spawn, pos, email, sprite, indeath, level, levelexp, resetcount, pk, data, vital, passresetcode, access
	    FROM public.player where uid = $1;
        "#)
            .bind(accountid)
            .fetch_one(conn),
    )?;

    player_with_id.into_player(world, entity);

    let player_inv: Vec<PGInvItem> = block_on(
        sqlx::query_as(
            r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.inventory where uid = $1;
        "#,
        )
        .bind(accountid)
        .fetch_all(conn),
    )?;

    PGInvItem::array_into_items(
        player_inv,
        &mut world.get::<&mut Inventory>(entity.0)?.items,
    );

    let player_eqpt: Vec<PGEquipItem> = block_on(
        sqlx::query_as(
            r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.equipment where uid = $1;
        "#,
        )
        .bind(accountid)
        .fetch_all(conn),
    )?;

    PGEquipItem::array_into_items(
        player_eqpt,
        &mut world.get::<&mut Equipment>(entity.0)?.items,
    );

    Ok(())
}

pub fn update_player(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    let player = PGPlayerLogOut::new(world, entity);

    block_on(
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
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_inv(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let inv = world.get_or_panic::<&Inventory>(entity);
    let account = world.get_or_panic::<&Account>(entity);
    let update = PGInvItem::single(&inv.items, account.id, slot).into_update();

    block_on(sqlx::query(&update).execute(conn))?;
    Ok(())
}

pub fn update_equipment(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let equip = world.get_or_panic::<&Equipment>(entity);
    let account = world.get_or_panic::<&Account>(entity);
    let update = PGEquipItem::single(&equip.items, account.id, slot).into_update();

    block_on(sqlx::query(&update).execute(conn))?;

    Ok(())
}

pub fn update_address(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    let player = PGPlayerAddress::new(world, entity);

    block_on(
        sqlx::query(
            r#"
                UPDATE public.player
                SET address=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.address)
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_playerdata(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    let player = PGPlayerData::new(world, entity);

    block_on(
        sqlx::query(
            r#"
                UPDATE public.player
                SET data=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.data)
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_passreset(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
    resetpassword: Option<String>,
) -> Result<()> {
    let player = PGPlayerPassReset::new(world, entity, resetpassword);

    block_on(
        sqlx::query(
            r#"
                UPDATE public.player
                SET passresetcode=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.passresetcode)
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_spawn(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    let player = PGPlayerSpawn::new(world, entity);

    block_on(
        sqlx::query(
            r#"
                UPDATE public.player
                SET spawn=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.spawn)
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_pos(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    let player = PGPlayerPos::new(world, entity);

    block_on(
        sqlx::query(
            r#"
                UPDATE public.player
                SET pos=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.pos)
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_currency(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    let player = PGPlayerCurrency::new(world, entity);

    block_on(
        sqlx::query(
            r#"
                UPDATE public.player
                SET vals=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.vals)
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_level(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    let player = PGPlayerLevel::new(world, entity);

    block_on(
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
        .execute(conn),
    )?;

    Ok(())
}

pub fn update_resetcount(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    let player = PGPlayerReset::new(world, entity);

    block_on(
        sqlx::query(
            r#"
                UPDATE public.player
                SET resetcount=$2
                WHERE uid = $1;
            "#,
        )
        .bind(player.uid)
        .bind(player.resetcount)
        .execute(conn),
    )?;

    Ok(())
}
