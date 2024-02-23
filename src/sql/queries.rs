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
            &r#"
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
        sqlx::query_as(&r#"SELECT EXISTS(SELECT 1 FROM player WHERE username=$1) as check"#)
            .bind(username)
            .fetch_one(conn),
    )?;

    if check.check {
        return Ok(1);
    };

    let check: Check = block_on(
        sqlx::query_as(&r#"SELECT EXISTS(SELECT 1 FROM player WHERE email=$1) as check"#)
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
    let uid: i64 = insert_into(players::table)
        .values(&PGPlayer::new(world, entity, username, email, password))
        .returning(players::uid)
        .get_result(conn)?;

    let inv = world.get_or_panic::<&Inventory>(entity);
    let equip = world.get_or_panic::<&Equipment>(entity);

    insert_into(equipment::table)
        .values(&PGEquipItem::new(&equip.items, uid))
        .execute(conn)?;

    insert_into(invitems::table)
        .values(&PGInvItem::new(&inv.items, uid))
        .execute(conn)?;

    Ok(uid)
}

pub fn load_player(
    _: &Storage,
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    let accountid = world.get_or_panic::<&Account>(entity).id;

    player_ret::table
        .filter(player_ret::uid.eq(accountid))
        .first::<PGPlayerWithID>(conn)?
        .into_player(world, entity);

    let mut equip = world.get_or_panic::<&Equipment>(entity).items.clone();
    PGEquipItem::array_into_items(
        equipment::table
            .filter(equipment::uid.eq(accountid))
            .load::<PGEquipItem>(conn)?,
        &mut equip,
    );

    let mut inv = world.get_or_panic::<&Equipment>(entity).items.clone();
    PGInvItem::array_into_items(
        invitems::table
            .filter(invitems::uid.eq(accountid))
            .load::<PGInvItem>(conn)?,
        &mut inv,
    );

    Ok(())
}

pub fn update_player(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerLogOut::new(world, entity))
        .execute(conn)?;
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

    diesel::update(invitems::table)
        .set(&PGInvItem::single(&inv.items, account.id, slot))
        .execute(conn)?;
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

    diesel::update(equipment::table)
        .set(&PGEquipItem::single(&equip.items, account.id, slot))
        .execute(conn)?;
    Ok(())
}

pub fn update_address(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerAddress::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_playerdata(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerData::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_passreset(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
    resetpassword: Option<String>,
) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerPassReset::new(world, entity, resetpassword))
        .execute(conn)?;
    Ok(())
}

pub fn update_spawn(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerSpawn::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_pos(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerPos::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_currency(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerCurrency::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_level(conn: &PgPool, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerLevel::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_resetcount(
    conn: &PgPool,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerReset::new(world, entity))
        .execute(conn)?;
    Ok(())
}
