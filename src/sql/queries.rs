use crate::{containers::*, gametypes::*, players::*, sql::integers::Shifting, sql::*};
use log::error;
use password_hash::SaltString;
use sqlx::{FromRow, PgPool};
use std::backtrace::Backtrace;
use tokio::sync::mpsc::*;

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct Check {
    pub check: bool,
}

//Used for SQL Thread Update Requests.
pub enum SqlRequests {
    Player(GlobalKey),
    Inv((GlobalKey, usize)),
    Storage((GlobalKey, usize)),
    Equipment((GlobalKey, usize)),
    Address(GlobalKey),
    PlayerData(GlobalKey),
    PassReset((GlobalKey, Option<String>)),
    Spawn(GlobalKey),
    Position(GlobalKey),
    Currency(GlobalKey),
    Level(GlobalKey),
    ResetCount(GlobalKey),
}

pub async fn process_sql_requests(
    world: GameWorld,
    storage: GameStore,
    mut rx: Receiver<SqlRequests>,
) -> Result<()> {
    loop {
        if let Some(request) = rx.recv().await {
            let res = match request {
                SqlRequests::Player(entity) => update_player(&storage, &world, &entity).await,
                SqlRequests::Inv((entity, slot)) => {
                    update_inv(&storage, &world, &entity, slot).await
                }
                SqlRequests::Storage((entity, slot)) => {
                    update_storage(&storage, &world, &entity, slot).await
                }
                SqlRequests::Equipment((entity, slot)) => {
                    update_equipment(&storage, &world, &entity, slot).await
                }
                SqlRequests::Address(entity) => update_address(&storage, &world, &entity).await,
                SqlRequests::PlayerData(entity) => {
                    update_playerdata(&storage, &world, &entity).await
                }
                SqlRequests::PassReset((entity, resetpassword)) => {
                    update_passreset(&storage, &world, &entity, resetpassword).await
                }
                SqlRequests::Spawn(entity) => update_spawn(&storage, &world, &entity).await,
                SqlRequests::Position(entity) => update_pos(&storage, &world, &entity).await,
                SqlRequests::Currency(entity) => update_currency(&storage, &world, &entity).await,
                SqlRequests::Level(entity) => update_level(&storage, &world, &entity).await,
                SqlRequests::ResetCount(entity) => {
                    update_resetcount(&storage, &world, &entity).await
                }
            };

            if let Err(e) = res {
                error!("process sql error: {e}");
            }
        }
    }

    //Ok(())
}

pub async fn load_player(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
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

    let lock = world.write().await;
    let mut inv_items = lock.get::<&mut Inventory>(entity.0)?;
    PGInvItem::array_into_items(player_inv, &mut inv_items.items);

    let player_storage: Vec<PGStorageItem> = sqlx::query_as(
        r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.storage where uid = $1;
        "#,
    )
    .bind(accountid)
    .fetch_all(&storage.pgconn)
    .await?;

    let mut inv_storage = lock.get::<&mut PlayerStorage>(entity.0)?;
    PGStorageItem::array_into_items(player_storage, &mut inv_storage.items);

    let player_eqpt: Vec<PGEquipItem> = sqlx::query_as(
        r#"
        SELECT uid, id, num, val, itemlevel, data
	    FROM public.equipment where uid = $1;
        "#,
    )
    .bind(accountid)
    .fetch_all(&storage.pgconn)
    .await?;

    let mut equipment = lock.get::<&mut Equipment>(entity.0)?;
    PGEquipItem::array_into_items(player_eqpt, &mut equipment.items);

    Ok(())
}

pub async fn update_player(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, itemtimer, position, vitals, death_timer, death_type, player) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(
            &Account,
            &PlayerItemTimer,
            &Position,
            &Vitals,
            &DeathTimer,
            &Death,
            &Player,
        )>(entity.0)?;
        let data =
            if let Some((account, itemtimer, position, vitals, death_timer, death_type, player)) =
                query.get()
            {
                (
                    account.id,
                    *itemtimer,
                    *position,
                    *vitals,
                    *death_timer,
                    *death_type,
                    *player,
                )
            } else {
                return Ok(());
            };

        data
    };

    sqlx::query(
        r#"
    UPDATE public.player
    SET itemtimer=$2, deathtimer=$3, pos=$4, vital=$5, indeath=$6, pk=$7, vital_max=$8
    WHERE uid = $1;
"#,
    )
    .bind(id)
    .bind(itemtimer.itemtimer)
    .bind(death_timer.0)
    .bind(position)
    .bind(vitals.vital)
    .bind(death_type.is_spirit())
    .bind(player.pk)
    .bind(vitals.vitalmax)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_inv(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
    slot: usize,
) -> Result<()> {
    let (inv, id) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Inventory, &Account)>(entity.0)?;
        let data = if let Some((inv, account)) = query.get() {
            (inv.clone(), account.id)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(&PGInvItem::single(&inv.items, id, slot).into_update())
        .execute(&storage.pgconn)
        .await?;

    Ok(())
}

pub async fn update_storage(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
    slot: usize,
) -> Result<()> {
    let (player_storage, id) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&PlayerStorage, &Account)>(entity.0)?;
        let data = if let Some((player_storage, account)) = query.get() {
            (player_storage.clone(), account.id)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(&PGStorageItem::single(&player_storage.items, id, slot).into_update())
        .execute(&storage.pgconn)
        .await?;

    Ok(())
}

pub async fn update_equipment(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
    slot: usize,
) -> Result<()> {
    let (equip, id) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Equipment, &Account)>(entity.0)?;
        let data = if let Some((equip, account)) = query.get() {
            (equip.clone(), account.id)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(&PGEquipItem::single(&equip.items, id, slot).into_update())
        .execute(&storage.pgconn)
        .await?;

    Ok(())
}

pub async fn update_address(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, addr) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Account, &Socket)>(entity.0)?;
        let data = if let Some((account, socket)) = query.get() {
            (account.id, socket.addr.clone())
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(
        r#"
        UPDATE public.player
        SET address=$2
        WHERE uid = $1;
    "#,
    )
    .bind(id)
    .bind(&*addr)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_playerdata(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, entity_data) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Account, &EntityData)>(entity.0)?;
        let data = if let Some((account, entity_data)) = query.get() {
            (account.id, *entity_data)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(
        r#"
        UPDATE public.player
        SET data=$2
        WHERE uid = $1;
    "#,
    )
    .bind(id)
    .bind(entity_data.0)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_passreset(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
    resetpassword: Option<String>,
) -> Result<()> {
    let id = {
        let lock = world.write().await;
        let id = if let Ok(account) = lock.get::<&Account>(entity.0) {
            account.id
        } else {
            return Ok(());
        };

        id
    };

    sqlx::query(
        r#"
                UPDATE public.player
                SET passresetcode=$2
                WHERE uid = $1;
            "#,
    )
    .bind(id)
    .bind(resetpassword)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_spawn(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, spawn) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Account, &Spawn)>(entity.0)?;
        let data = if let Some((account, spawn)) = query.get() {
            (account.id, *spawn)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(
        r#"
        UPDATE public.player
        SET spawn=$2
        WHERE uid = $1;
    "#,
    )
    .bind(id)
    .bind(spawn.pos)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_pos(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, position) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Account, &Position)>(entity.0)?;
        let data = if let Some((account, position)) = query.get() {
            (account.id, *position)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(
        r#"
        UPDATE public.player
        SET pos=$2
        WHERE uid = $1;
    "#,
    )
    .bind(id)
    .bind(position)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_currency(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, money) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Account, &Money)>(entity.0)?;
        let data = if let Some((account, money)) = query.get() {
            (account.id, *money)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(
        r#"
        UPDATE public.player
        SET vals=$2
        WHERE uid = $1;
    "#,
    )
    .bind(id)
    .bind(i64::unshift_signed(&money.vals))
    .execute(&storage.pgconn)
    .await?;
    Ok(())
}

pub async fn update_level(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, level, player, vitals) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Account, &Level, &Player, &Vitals)>(entity.0)?;
        let data = if let Some((account, level, player, vitals)) = query.get() {
            (account.id, *level, *player, *vitals)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(
        r#"
                UPDATE public.player
                SET level=$2, levelexp=$3, vital=$4, vital_max=$5
                WHERE uid = $1;
            "#,
    )
    .bind(id)
    .bind(level.0)
    .bind(i64::unshift_signed(&player.levelexp))
    .bind(vitals.vital)
    .bind(vitals.vitalmax)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}

pub async fn update_resetcount(
    storage: &GameStore,
    world: &GameWorld,
    entity: &crate::GlobalKey,
) -> Result<()> {
    let (id, player) = {
        let lock = world.write().await;
        let mut query = lock.query_one::<(&Account, &Player)>(entity.0)?;
        let data = if let Some((account, player)) = query.get() {
            (account.id, *player)
        } else {
            return Ok(());
        };

        data
    };

    sqlx::query(
        r#"
            UPDATE public.player
            SET resetcount=$2
            WHERE uid = $1;
        "#,
    )
    .bind(id)
    .bind(player.resetcount)
    .execute(&storage.pgconn)
    .await?;

    Ok(())
}
