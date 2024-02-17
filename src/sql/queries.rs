use crate::{containers::*, players::*, sql::*, AscendingError, Result};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use diesel::{self, insert_into, prelude::*};

pub fn find_player(conn: &mut PgConnection, username: &str, password: &str) -> Result<Option<i64>> {
    let userdata = players::table
        .filter(players::username.eq(username))
        .select((players::uid, players::password))
        .first::<PlayerWithPassword>(conn)
        .optional()?;

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

pub fn check_existance(
    conn: &mut PgConnection,
    username: &str,
    name: &str,
    email: &str,
) -> Result<i64> {
    if let Some(_id) = players::table
        .filter(players::username.eq(username))
        .select(players::uid)
        .first::<i64>(conn)
        .optional()?
    {
        return Ok(1);
    };

    if let Some(_id) = players::table
        .filter(players::name.eq(name))
        .select(players::uid)
        .first::<i64>(conn)
        .optional()?
    {
        return Ok(2);
    };

    if let Some(_id) = players::table
        .filter(players::email.eq(email))
        .select(players::uid)
        .first::<i64>(conn)
        .optional()?
    {
        return Ok(3);
    };

    Ok(0)
}

pub fn new_player(
    conn: &mut PgConnection,
    world: &mut hecs::World,
    entity: &crate::Entity,
    username: String,
    email: String,
    password: String,
) -> Result<()> {
    let uid: i64 = insert_into(players::table)
        .values(&PGPlayer::new(world, entity, username, email, password))
        .returning(players::uid)
        .get_result(conn)?;

    let data = world.entity(entity.0).expect("Could not get Entity");
    let inv = data.get::<&Inventory>().expect("Could not find Inventory");
    let equip = data.get::<&Equipment>().expect("Could not find Equipment");

    insert_into(equipment::table)
        .values(&PGEquipItem::new(&equip.items, uid))
        .execute(conn)?;

    insert_into(invitems::table)
        .values(&PGInvItem::new(&inv.items, uid))
        .execute(conn)?;

    Ok(())
}

pub fn load_player(
    _: &Storage,
    conn: &mut PgConnection,
    world: &mut hecs::World,
    entity: &crate::Entity,
) -> Result<()> {
    let accountid = world.get::<&Account>(entity.0).expect("Could not find Account").id.clone();

    player_ret::table
        .filter(player_ret::uid.eq(accountid))
        .first::<PGPlayerWithID>(conn)?
        .into_player(world, entity);

    let mut equip = world.get::<&Equipment>(entity.0).expect("Could not find Equipment").items.clone();
    PGEquipItem::array_into_items(
        equipment::table
            .filter(equipment::uid.eq(accountid))
            .load::<PGEquipItem>(conn)?,
        &mut equip,
    );

    let mut inv = world.get::<&Equipment>(entity.0).expect("Could not find Equipment").items.clone();
    PGInvItem::array_into_items(
        invitems::table
            .filter(invitems::uid.eq(accountid))
            .load::<PGInvItem>(conn)?,
        &mut inv,
    );

    Ok(())
}

pub fn update_player(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerLogOut::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_inv(
    conn: &mut PgConnection,
    world: &mut hecs::World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let data = world.entity(entity.0).expect("Could not get Entity");
    let inv = data.get::<&Inventory>().expect("Could not find Inventory");
    let account = data.get::<&Account>().expect("Could not find Account");

    diesel::update(invitems::table)
        .set(&PGInvItem::single(&inv.items, account.id, slot))
        .execute(conn)?;
    Ok(())
}

pub fn update_equipment(
    conn: &mut PgConnection,
    world: &mut hecs::World,
    entity: &crate::Entity,
    slot: usize,
) -> Result<()> {
    let data = world.entity(entity.0).expect("Could not get Entity");
    let equip = data.get::<&Equipment>().expect("Could not find Equipment");
    let account = data.get::<&Account>().expect("Could not find Account");

    diesel::update(equipment::table)
        .set(&PGEquipItem::single(&equip.items, account.id, slot))
        .execute(conn)?;
    Ok(())
}

pub fn update_address(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerAddress::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_playerdata(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerData::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_passreset(
    conn: &mut PgConnection,
    world: &mut hecs::World,
    entity: &crate::Entity,
    resetpassword: Option<String>,
) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerPassReset::new(world, entity, resetpassword))
        .execute(conn)?;
    Ok(())
}

pub fn update_spawn(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerSpawn::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_pos(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerPos::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_currency(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerCurrency::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_level(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerLevel::new(world, entity))
        .execute(conn)?;
    Ok(())
}

pub fn update_resetcount(conn: &mut PgConnection, world: &mut hecs::World, entity: &crate::Entity) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerReset::new(world, entity))
        .execute(conn)?;
    Ok(())
}
