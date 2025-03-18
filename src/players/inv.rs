use crate::{
    containers::*,
    gametypes::*,
    items::*,
    players::*,
    socket::*,
    sql::*,
    tasks::{DataTaskToken, damage_packet},
};

#[inline]
pub fn save_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    slot: usize,
) -> Result<()> {
    update_inv(storage, world, entity, slot)?;
    send_invslot(world, storage, entity, slot)
}

#[inline]
pub fn count_inv_item(num: u32, inv: &[Item]) -> u64 {
    (0..MAX_INV)
        .filter_map(|id| {
            if inv[id].num == num && inv[id].val > 0 {
                Some(inv[id].val as u64)
            } else {
                None
            }
        })
        .fold(0u64, u64::saturating_add)
}

#[inline]
pub fn find_inv_item(num: u32, inv: &[Item]) -> Option<usize> {
    (0..MAX_INV).find(|id| inv[*id].num == num && inv[*id].val > 0)
}

#[inline]
pub fn find_inv_slot(item: &Item, inv: &[Item], base: &ItemData) -> Option<usize> {
    if base.stackable {
        if let Some(id) = (0..MAX_INV).find(|id| {
            inv[*id].num == item.num && inv[*id].val < base.stacklimit && inv[*id].val > 0
        }) {
            return Some(id);
        }
    }

    (0..MAX_INV).find(|id| inv[*id].val == 0)
}

#[inline]
pub fn auto_set_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<()> {
    let mut save_item_list = Vec::new();
    let mut total_left = if item.val == 0 { 1 } else { item.val };

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        if base.stackable {
            for id in 0..MAX_INV {
                if p_data.inventory.items[id].num == item.num
                    && p_data.inventory.items[id].val < base.stacklimit
                    && p_data.inventory.items[id].val > 0
                {
                    val_add_rem(
                        &mut p_data.inventory.items[id].val,
                        &mut total_left,
                        base.stacklimit,
                    );

                    save_item_list.push(id);

                    if total_left == 0 {
                        break;
                    }
                }
            }
        }

        item.val = total_left;

        if total_left != 0 {
            for id in 0..MAX_INV {
                if p_data.inventory.items[id].val == 0 {
                    p_data.inventory.items[id] = *item;
                    item.val = 0;
                    save_item_list.push(id);
                    break;
                }
            }
        }
    }

    for slot in save_item_list.iter() {
        save_inv_item(world, storage, entity, *slot)?;
    }

    Ok(())
}

pub fn check_inv_item_space(
    world: &mut World,
    entity: GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<bool> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        let mut total_left = if item.val == 0 { 1 } else { item.val };
        let mut empty_space_count = 0;

        //First try to add it to other of the same type
        for id in 0..MAX_INV {
            if base.stackable
                && p_data.inventory.items[id].num == item.num
                && p_data.inventory.items[id].val < base.stacklimit
                && p_data.inventory.items[id].val > 0
            {
                if p_data.inventory.items[id].val + total_left > base.stacklimit {
                    total_left = total_left + p_data.inventory.items[id].val - base.stacklimit;
                } else {
                    return Ok(true);
                }
            } else if p_data.inventory.items[id].val == 0 {
                if !base.stackable {
                    return Ok(true);
                }

                empty_space_count += 1;
            }
        }

        Ok(empty_space_count > 0)
    } else {
        Ok(false)
    }
}

pub fn check_inv_item_partial_space(
    world: &mut World,
    entity: GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<(u16, u16)> {
    let mut total_left = if item.val == 0 { 1 } else { item.val };
    let start_val = if item.val == 0 { 1 } else { item.val };

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        //First try to add it to other of the same type
        if base.stackable {
            for id in 0..MAX_INV {
                if p_data.inventory.items[id].num == item.num
                    && p_data.inventory.items[id].val < base.stacklimit
                    && p_data.inventory.items[id].val > 0
                {
                    if p_data.inventory.items[id].val + total_left > base.stacklimit {
                        total_left = total_left + p_data.inventory.items[id].val - base.stacklimit;
                    } else {
                        return Ok((0, start_val));
                    }
                }
            }
        }

        for id in 0..MAX_INV {
            if p_data.inventory.items[id].val == 0 {
                return Ok((0, start_val));
            }
        }
    }

    Ok((total_left, start_val))
}

#[allow(clippy::too_many_arguments)]
#[inline]
pub fn set_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: &mut Item,
    base: &ItemData,
    slot: usize,
    amount: u16,
) -> Result<u16> {
    let mut rem = 0u16;

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let player_inv = { p_data.try_lock()?.inventory.items[slot] };

        let item_min = std::cmp::min(amount, item.val);

        if player_inv.val == 0 {
            {
                let mut p_data = p_data.try_lock()?;

                p_data.inventory.items[slot] = *item;
                p_data.inventory.items[slot].val = item_min;
            }

            save_inv_item(world, storage, entity, slot)?;
            return Ok(0);
        } else if player_inv.num == item.num {
            {
                let mut p_data = p_data.try_lock()?;

                rem = val_add_amount_rem(
                    &mut p_data.inventory.items[slot].val,
                    &mut item.val,
                    item_min,
                    base.stacklimit,
                );
            }

            save_inv_item(world, storage, entity, slot)?;
        }
    }

    Ok(rem)
}

#[inline]
pub fn give_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: &mut Item,
) -> Result<()> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_inv_item(world, storage, entity, item, base)
}

pub fn check_inv_space(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: &mut Item,
) -> Result<bool> {
    let base = &storage.bases.items[item.num as usize];

    check_inv_item_space(world, entity, item, base)
}

//checks if we only got partial or not if so returns how many we got.
//Returns is_less, amount removed, amount it started with.
pub fn check_inv_partial_space(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: &mut Item,
) -> Result<(bool, u16, u16)> {
    let base = &storage.bases.items[item.num as usize];

    let (left, start) = check_inv_item_partial_space(world, entity, item, base)?;

    if left < start {
        Ok((true, start - left, start))
    } else {
        Ok((false, start, 0))
    }
}

#[inline]
pub fn set_inv_slot(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: &mut Item,
    slot: usize,
    amount: u16,
) -> Result<u16> {
    let base = &storage.bases.items[item.num as usize];

    set_inv_item(world, storage, entity, item, base, slot, amount)
}

#[inline]
pub fn take_inv_items(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    num: u32,
    mut amount: u16,
) -> Result<u16> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let inventory = { p_data.try_lock()?.inventory.clone() };

        if count_inv_item(num, &inventory.items) >= amount as u64 {
            while let Some(slot) = find_inv_item(num, &inventory.items) {
                {
                    let mut p_data = p_data.try_lock()?;

                    let take_amount = p_data.inventory.items[slot].val;
                    p_data.inventory.items[slot].val =
                        p_data.inventory.items[slot].val.saturating_sub(amount);

                    amount = amount.saturating_sub(take_amount);
                }

                save_inv_item(world, storage, entity, slot)?;

                if amount == 0 {
                    return Ok(0);
                }
            }
        }
    }

    Ok(amount)
}

#[inline]
pub fn take_inv_itemslot(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    slot: usize,
    mut amount: u16,
) -> Result<u16> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let result = {
            let mut p_data = p_data.try_lock()?;

            amount = std::cmp::min(amount, p_data.inventory.items[slot].val);

            p_data.inventory.items[slot].val =
                p_data.inventory.items[slot].val.saturating_sub(amount);
            if p_data.inventory.items[slot].val == 0 {
                p_data.inventory.items[slot] = Item::default();
            }

            p_data.inventory.items[slot].val
        };

        save_inv_item(world, storage, entity, slot)?;

        return Ok(result);
    }
    Ok(0)
}

#[inline]
pub fn count_trade_item(num: u32, trade_slot: &[Item]) -> u64 {
    (0..MAX_TRADE_SLOT)
        .filter_map(|id| {
            if trade_slot[id].num == num && trade_slot[id].val > 0 {
                Some(trade_slot[id].val as u64)
            } else {
                None
            }
        })
        .fold(0u64, u64::saturating_add)
}

#[inline]
pub fn find_trade_slot(item: &Item, trade_slot: &[Item], base: &ItemData) -> Option<usize> {
    if base.stackable {
        if let Some(id) = (0..MAX_INV).find(|id| {
            trade_slot[*id].num == item.num
                && trade_slot[*id].val < base.stacklimit
                && trade_slot[*id].val > 0
        }) {
            return Some(id);
        }
    }

    (0..MAX_INV).find(|id| trade_slot[*id].val == 0)
}

#[inline]
pub fn auto_set_trade_item(
    world: &mut World,
    entity: GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<Vec<usize>> {
    let mut save_slot_list = Vec::new();

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        while let Some(slot) = find_trade_slot(item, &p_data.trade_item.items, base) {
            if p_data.trade_item.items[slot].val == 0 {
                p_data.trade_item.items[slot] = *item;
                item.val = 0;
                save_slot_list.push(slot);
                break;
            }

            let rem = val_add_rem(
                &mut p_data.trade_item.items[slot].val,
                &mut item.val,
                base.stacklimit,
            );
            save_slot_list.push(slot);

            if rem == 0 {
                break;
            }
        }
    }

    Ok(save_slot_list)
}

#[inline]
pub fn give_trade_item(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: &mut Item,
) -> Result<Vec<usize>> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_trade_item(world, entity, item, base)
}

pub fn check_temp_inv_space(
    storage: &Storage,
    item: &mut Item,
    temp_inv: &mut Inventory,
) -> Result<bool> {
    let base = &storage.bases.items[item.num as usize];

    check_temp_inv_item_space(item, base, temp_inv)
}

pub fn check_temp_inv_item_space(
    item: &mut Item,
    base: &ItemData,
    temp_inv: &mut Inventory,
) -> Result<bool> {
    let mut total_left = if item.val == 0 { 1 } else { item.val };
    let mut empty_space_count = 0;

    //First try to add it to other of the same type
    for id in 0..MAX_INV {
        if base.stackable
            && temp_inv.items[id].num == item.num
            && temp_inv.items[id].val < base.stacklimit
            && temp_inv.items[id].val > 0
        {
            if temp_inv.items[id].val + total_left > base.stacklimit {
                total_left = total_left + temp_inv.items[id].val - base.stacklimit;
            } else {
                return Ok(true);
            }
        } else if temp_inv.items[id].val == 0 {
            if !base.stackable {
                return Ok(true);
            }

            empty_space_count += 1;
        }
    }

    Ok(empty_space_count > 0)
}

#[inline]
pub fn auto_set_temp_inv_item(
    item: &mut Item,
    base: &ItemData,
    temp_inv: &mut Inventory,
) -> Result<()> {
    let mut total_left = if item.val == 0 { 1 } else { item.val };

    {
        if base.stackable {
            for id in 0..MAX_INV {
                if temp_inv.items[id].num == item.num
                    && temp_inv.items[id].val < base.stacklimit
                    && temp_inv.items[id].val > 0
                {
                    val_add_rem(
                        &mut temp_inv.items[id].val,
                        &mut total_left,
                        base.stacklimit,
                    );

                    if total_left == 0 {
                        break;
                    }
                }
            }
        }

        item.val = total_left;

        if total_left != 0 {
            for id in 0..MAX_INV {
                if temp_inv.items[id].val == 0 {
                    temp_inv.items[id] = *item;
                    item.val = 0;
                    break;
                }
            }
        }
    }

    Ok(())
}

#[inline]
pub fn give_temp_inv_item(
    storage: &Storage,
    item: &mut Item,
    temp_inv: &mut Inventory,
) -> Result<()> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_temp_inv_item(item, base, temp_inv)
}

pub fn player_unequip(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    slot: usize,
) -> Result<bool> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut item = {
            let p_data = p_data.try_lock()?;

            if p_data.equipment.items[slot].val == 0 {
                return Ok(true);
            }

            p_data.equipment.items[slot]
        };

        if !check_inv_space(world, storage, entity, &mut item)? {
            return Ok(false);
        }

        give_inv_item(world, storage, entity, &mut item)?;

        {
            p_data.try_lock()?.equipment.items[slot] = Item::default();
        }

        update_equipment(storage, world, entity, slot)?;
        send_equipment(world, storage, entity)?;

        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn player_equip(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item: Item,
    slot: usize,
) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        {
            p_data.try_lock()?.equipment.items[slot] = item;
        }
        update_equipment(storage, world, entity, slot)?;
        send_equipment(world, storage, entity)?;
    }
    Ok(())
}

pub fn player_use_item(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    slot: u16,
) -> Result<()> {
    if slot as usize >= MAX_INV {
        return Ok(());
    }

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let (item, player_pos, player_vital) = {
            let p_data = p_data.try_lock()?;

            (
                p_data.inventory.items[slot as usize],
                p_data.movement.pos,
                p_data.combat.vitals,
            )
        };

        if item.val == 0 {
            return Ok(());
        }

        let base = &storage.bases.items[item.num as usize];

        match base.itemtype {
            ItemTypes::Consume => {
                if base.data[0] > 0 {
                    let set_vital = player_vital.vital[VitalTypes::Hp as usize]
                        .saturating_add(base.data[0] as i32)
                        .min(player_vital.vitalmax[VitalTypes::Hp as usize]);
                    player_set_vital(world, storage, entity, VitalTypes::Hp, set_vital)?;

                    DataTaskToken::Damage(player_pos.map).add_task(
                        storage,
                        damage_packet(entity, base.data[0] as u16, player_pos, false)?,
                    )?;
                }

                if base.data[1] > 0 {
                    let set_vital = player_vital.vital[VitalTypes::Mp as usize]
                        .saturating_add(base.data[1] as i32)
                        .min(player_vital.vitalmax[VitalTypes::Mp as usize]);
                    player_set_vital(world, storage, entity, VitalTypes::Mp, set_vital)?;
                }

                if base.data[2] > 0 {
                    let set_vital = player_vital.vital[VitalTypes::Sp as usize]
                        .saturating_add(base.data[2] as i32)
                        .min(player_vital.vitalmax[VitalTypes::Sp as usize]);
                    player_set_vital(world, storage, entity, VitalTypes::Sp, set_vital)?;
                }
            }
            ItemTypes::Weapon
            | ItemTypes::Helmet
            | ItemTypes::Armor
            | ItemTypes::Trouser
            | ItemTypes::Accessory => {
                let eqslot = match base.itemtype {
                    ItemTypes::Helmet => EquipmentType::Helmet,
                    ItemTypes::Armor => EquipmentType::Chest,
                    ItemTypes::Trouser => EquipmentType::Pants,
                    ItemTypes::Accessory => EquipmentType::Accessory,
                    _ => EquipmentType::Weapon,
                } as usize;

                if !player_unequip(world, storage, entity, eqslot)? {
                    // ToDo Warning cannot unequip
                    return Ok(());
                }
                player_equip(world, storage, entity, item, eqslot)?;
            }
            _ => return Ok(()),
        }

        if let Some(_sfx) = &base.sound_index {
            send_playitemsfx(world, storage, entity, item.num as u16)?;
        }

        take_inv_itemslot(world, storage, entity, slot as usize, 1)?;
    }
    Ok(())
}
