use super::{
    PGCombat, PGEquipmentSlot, PGGeneral, PGInventorySlot, PGLocation, PGStorageSlot,
    sql_update_combat, sql_update_equipment_slot, sql_update_general, sql_update_inventory_slot,
    sql_update_level, sql_update_location, sql_update_money, sql_update_resetcount,
    sql_update_storage_slot,
};
use crate::{
    containers::{Entity, GlobalKey, Storage, World},
    gametypes::*,
    sql::integers::Shifting,
};
use time::Instant;

pub fn get_time_left(cur_time: Instant, system_time: Instant) -> i64 {
    let cur_timer = cur_time.to_dur();
    let cur_time = system_time.to_dur();
    cur_timer.saturating_sub(cur_time).max(0)
}

pub fn update_player(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    let tick = *storage.gettick.borrow();

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        sql_update_combat(
            storage,
            p_data.account.id,
            PGCombat {
                level: p_data.combat.level,
                levelexp: i64::unshift_signed(&p_data.general.levelexp),
                vital: p_data.combat.vitals.vital,
                vital_max: p_data.combat.vitals.vitalmax,
                indeath: false,
                pk: p_data.general.pk,
            },
        )?;

        sql_update_general(
            storage,
            p_data.account.id,
            PGGeneral {
                sprite: i16::unshift_signed(&p_data.sprite.id),
                money: i64::unshift_signed(&p_data.money.vals),
                resetcount: p_data.general.resetcount,
                itemtimer: get_time_left(p_data.item_timer.itemtimer, tick),
                deathtimer: get_time_left(p_data.combat.death_timer.0, tick),
            },
        )?;

        sql_update_location(
            storage,
            p_data.account.id,
            PGLocation {
                spawn: p_data.movement.spawn.pos,
                pos: p_data.movement.pos,
                dir: p_data.movement.dir as i16,
            },
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
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        let uid = p_data.account.id;

        if let Some(slot_data) = p_data.inventory.items.get(slot) {
            sql_update_inventory_slot(
                storage,
                uid,
                PGInventorySlot {
                    id: slot as i16,
                    num: i32::unshift_signed(&slot_data.num),
                    val: i16::unshift_signed(&slot_data.val),
                    level: slot_data.level as i16,
                    data: slot_data.data,
                },
            )?;
        }
    }

    Ok(())
}

pub fn update_storage(
    storage: &Storage,
    world: &mut World,
    entity: GlobalKey,
    slot: usize,
) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        let uid = p_data.account.id;

        if let Some(slot_data) = p_data.storage.items.get(slot) {
            sql_update_storage_slot(
                storage,
                uid,
                PGStorageSlot {
                    id: slot as i16,
                    num: i32::unshift_signed(&slot_data.num),
                    val: i16::unshift_signed(&slot_data.val),
                    level: slot_data.level as i16,
                    data: slot_data.data,
                },
            )?;
        }
    }

    Ok(())
}

pub fn update_equipment(
    storage: &Storage,
    world: &mut World,
    entity: GlobalKey,
    slot: usize,
) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        let uid = p_data.account.id;

        if let Some(slot_data) = p_data.equipment.items.get(slot) {
            sql_update_equipment_slot(
                storage,
                uid,
                PGEquipmentSlot {
                    id: slot as i16,
                    num: i32::unshift_signed(&slot_data.num),
                    val: i16::unshift_signed(&slot_data.val),
                    level: slot_data.level as i16,
                    data: slot_data.data,
                },
            )?;
        }
    }

    Ok(())
}

pub fn update_pos(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        sql_update_location(
            storage,
            p_data.account.id,
            PGLocation {
                spawn: p_data.movement.spawn.pos,
                pos: p_data.movement.pos,
                dir: p_data.movement.dir as i16,
            },
        )?;
    }

    Ok(())
}

pub fn update_currency(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        sql_update_money(
            storage,
            p_data.account.id,
            i64::unshift_signed(&p_data.money.vals),
        )?;
    }

    Ok(())
}

pub fn update_level(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        sql_update_level(
            storage,
            p_data.account.id,
            PGCombat {
                level: p_data.combat.level,
                levelexp: i64::unshift_signed(&p_data.general.levelexp),
                vital: p_data.combat.vitals.vital,
                vital_max: p_data.combat.vitals.vitalmax,
                ..Default::default()
            },
        )?;
    }
    Ok(())
}

pub fn update_resetcount(storage: &Storage, world: &mut World, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        sql_update_resetcount(storage, p_data.account.id, p_data.general.resetcount)?;
    }
    Ok(())
}
