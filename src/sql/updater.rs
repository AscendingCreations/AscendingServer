use crate::{
    containers::{GlobalKey, Storage, World},
    gametypes::*,
};

pub fn update_player(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
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
        local.block_on(
            &rt,
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
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}

pub fn update_inv(
    storage: &Storage,
    world: &mut World,
    entity: GlobalKey,
    slot: usize,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Inventory, &Account)>(entity.0)?;
    if let Some((inv, account)) = query.get() {
        let update = PGInvItem::single(&inv.items, account.id, slot).into_update();

        local.block_on(&rt, sqlx::query(&update).execute(&storage.pgconn))?;
    }

    Ok(())
}

pub fn update_storage(
    storage: &Storage,
    world: &mut World,
    entity: GlobalKey,
    slot: usize,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&PlayerStorage, &Account)>(entity.0)?;
    if let Some((player_storage, account)) = query.get() {
        let update = PGStorageItem::single(&player_storage.items, account.id, slot).into_update();

        local.block_on(&rt, sqlx::query(&update).execute(&storage.pgconn))?;
    }

    Ok(())
}

pub fn update_equipment(
    storage: &Storage,
    world: &mut World,
    entity: GlobalKey,
    slot: usize,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Equipment, &Account)>(entity.0)?;
    if let Some((equip, account)) = query.get() {
        let update = PGEquipItem::single(&equip.items, account.id, slot).into_update();

        local.block_on(&rt, sqlx::query(&update).execute(&storage.pgconn))?;
    }

    Ok(())
}

pub fn update_address(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Account, &Socket)>(entity.0)?;
    if let Some((account, socket)) = query.get() {
        local.block_on(
            &rt,
            sqlx::query(
                r#"
                UPDATE public.player
                SET address=$2
                WHERE uid = $1;
            "#,
            )
            .bind(account.id)
            .bind(&*socket.addr)
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}

pub fn update_playerdata(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Account, &EntityData)>(entity.0)?;
    if let Some((account, entity_data)) = query.get() {
        local.block_on(
            &rt,
            sqlx::query(
                r#"
                UPDATE public.player
                SET data=$2
                WHERE uid = $1;
            "#,
            )
            .bind(account.id)
            .bind(entity_data.0)
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}

pub fn update_passreset(
    storage: &Storage,
    world: &mut World,
    entity: GlobalKey,
    resetpassword: Option<String>,
) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();

    local.block_on(
        &rt,
        sqlx::query(
            r#"
                UPDATE public.player
                SET passresetcode=$2
                WHERE uid = $1;
            "#,
        )
        .bind(world.get::<&Account>(entity.0)?.id)
        .bind(resetpassword)
        .execute(&storage.pgconn),
    )?;

    Ok(())
}

pub fn update_spawn(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Account, &Spawn)>(entity.0)?;
    if let Some((account, spawn)) = query.get() {
        local.block_on(
            &rt,
            sqlx::query(
                r#"
                UPDATE public.player
                SET spawn=$2
                WHERE uid = $1;
            "#,
            )
            .bind(account.id)
            .bind(spawn.pos)
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}

pub fn update_pos(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Account, &Position)>(entity.0)?;
    if let Some((account, position)) = query.get() {
        local.block_on(
            &rt,
            sqlx::query(
                r#"
                UPDATE public.player
                SET pos=$2
                WHERE uid = $1;
            "#,
            )
            .bind(account.id)
            .bind(position)
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}

pub fn update_currency(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Account, &Money)>(entity.0)?;
    if let Some((account, money)) = query.get() {
        local.block_on(
            &rt,
            sqlx::query(
                r#"
                UPDATE public.player
                SET vals=$2
                WHERE uid = $1;
            "#,
            )
            .bind(account.id)
            .bind(i64::unshift_signed(&money.vals))
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}

pub fn update_level(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Account, &Level, &Player, &Vitals)>(entity.0)?;
    if let Some((account, level, player, vitals)) = query.get() {
        local.block_on(
            &rt,
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
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}

pub fn update_resetcount(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let rt = storage.rt.borrow_mut();
    let local = storage.local.borrow();
    let mut query = world.query_one::<(&Account, &Player)>(entity.0)?;
    if let Some((account, player)) = query.get() {
        local.block_on(
            &rt,
            sqlx::query(
                r#"
                UPDATE public.player
                SET resetcount=$2
                WHERE uid = $1;
            "#,
            )
            .bind(account.id)
            .bind(player.resetcount)
            .execute(&storage.pgconn),
        )?;
    }

    Ok(())
}
