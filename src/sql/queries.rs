use crate::{containers::*, sql::*, AscendingError, Result};
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
    user: &crate::players::Player,
    username: String,
    email: String,
    password: String,
) -> Result<()> {
    let uid: i64 = insert_into(players::table)
        .values(&PGPlayer::new(user, username, email, password))
        .returning(players::uid)
        .get_result(conn)?;

    insert_into(equipment::table)
        .values(&PGEquipItem::new(&user.equip, uid))
        .execute(conn)?;

    insert_into(invitems::table)
        .values(&PGInvItem::new(&user.inv, uid))
        .execute(conn)?;

    Ok(())
}

pub fn load_player(
    _: &Storage,
    conn: &mut PgConnection,
    user: &mut crate::players::Player,
) -> Result<()> {
    player_ret::table
        .filter(player_ret::uid.eq(user.accid))
        .first::<PGPlayerWithID>(conn)?
        .into_player(user);

    PGEquipItem::array_into_items(
        equipment::table
            .filter(equipment::uid.eq(user.accid))
            .load::<PGEquipItem>(conn)?,
        &mut user.equip,
    );

    PGInvItem::array_into_items(
        invitems::table
            .filter(invitems::uid.eq(user.accid))
            .load::<PGInvItem>(conn)?,
        &mut user.inv,
    );

    Ok(())
}

pub fn update_player(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerLogOut::new(user))
        .execute(conn)?;
    Ok(())
}

pub fn update_inv(
    conn: &mut PgConnection,
    user: &mut crate::players::Player,
    slot: usize,
) -> Result<()> {
    diesel::update(invitems::table)
        .set(&PGInvItem::single(&user.inv, user.accid, slot))
        .execute(conn)?;
    Ok(())
}

pub fn update_equipment(
    conn: &mut PgConnection,
    user: &mut crate::players::Player,
    slot: usize,
) -> Result<()> {
    diesel::update(equipment::table)
        .set(&PGEquipItem::single(&user.equip, user.accid, slot))
        .execute(conn)?;
    Ok(())
}

pub fn update_address(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerAddress::new(user))
        .execute(conn)?;
    Ok(())
}

pub fn update_playerdata(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerData::new(user))
        .execute(conn)?;
    Ok(())
}

pub fn update_passreset(
    conn: &mut PgConnection,
    user: &mut crate::players::Player,
    resetpassword: Option<String>,
) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerPassReset::new(user, resetpassword))
        .execute(conn)?;
    Ok(())
}

pub fn update_spawn(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerSpawn::new(user))
        .execute(conn)?;
    Ok(())
}

pub fn update_pos(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerPos::new(user))
        .execute(conn)?;
    Ok(())
}

pub fn update_currency(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerCurrency::new(user))
        .execute(conn)?;
    Ok(())
}

pub fn update_level(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerLevel::new(user))
        .execute(conn)?;
    Ok(())
}

pub fn update_resetcount(conn: &mut PgConnection, user: &mut crate::players::Player) -> Result<()> {
    diesel::update(players::table)
        .set(&PGPlayerReset::new(user))
        .execute(conn)?;
    Ok(())
}
