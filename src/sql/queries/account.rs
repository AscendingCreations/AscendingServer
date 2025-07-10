use argon2::{Argon2, PasswordHasher};
use password_hash::SaltString;
use uuid::Uuid;

use crate::{
    containers::{SALT, Storage, UserAccess},
    gametypes::*,
};

use sqlx::FromRow;

#[derive(Debug, FromRow, Default)]
pub struct PGAccount {
    pub username: String,
    pub email: String,
    pub useraccess: UserAccess,
    pub passresetcode: Option<String>,
}

impl PGAccount {
    pub fn into_empty() -> String {
        r#"
            INSERT INTO public.account(username, address, password, email, passresetcode, useraccess)
            VALUES ($1, $2, $3, $4, null, 'None') RETURNING uid;
            "#.to_string()
    }
}

pub fn sql_new_account(
    storage: &Storage,
    username: &str,
    address: &str,
    password: &str,
    email: &str,
) -> Result<Uuid> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

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

    let query = PGAccount::into_empty();
    let result: (Uuid,) = local.block_on(
        &rt,
        sqlx::query_as(&query)
            .bind(username)
            .bind(address)
            .bind(hashed_password)
            .bind(email)
            .fetch_one(&storage.pgconn),
    )?;

    Ok(result.0)
}

pub fn sql_load_account(storage: &Storage, account_id: Uuid) -> Result<PGAccount> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query = format!(
        r#"
        SELECT username, email, passresetcode, useraccess
        FROM public.account
        WHERE uid = '{account_id}';
        "#,
    );
    let data: PGAccount = local.block_on(&rt, sqlx::query_as(&query).fetch_one(&storage.pgconn))?;

    Ok(data)
}

pub fn sql_update_account(storage: &Storage, uid: Uuid, user_access: UserAccess) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    let query_text = format!(
        r#"
        UPDATE public.account
        SET useraccess=$1
        WHERE uid = '{uid}';
        "#
    );

    local.block_on(
        &rt,
        sqlx::query(&query_text)
            .bind(user_access)
            .execute(&storage.pgconn),
    )?;

    Ok(())
}
