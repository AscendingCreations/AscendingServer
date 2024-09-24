use crate::{
    containers::*,
    gametypes::*,
    items::*,
    network::*,
    players::*,
    sql::*,
    tasks::{damage_packet, DataTaskToken},
};

#[inline]
pub async fn save_inv_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &GlobalKey,
    slot: usize,
) -> Result<()> {
    storage
        .sql_request
        .send(SqlRequests::Inv((*entity, slot)))
        .await?;
    send_invslot(world, storage, entity, slot).await
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
pub async fn auto_set_inv_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<()> {
    let mut save_item_list = Vec::new();
    let mut total_left = if item.val == 0 { 1 } else { item.val };

    {
        let lock = world.write().await;
        let mut player_inv = lock.get::<&mut Inventory>(entity.0)?;

        if base.stackable {
            for id in 0..MAX_INV {
                if player_inv.items[id].num == item.num
                    && player_inv.items[id].val < base.stacklimit
                    && player_inv.items[id].val > 0
                {
                    val_add_rem(
                        &mut player_inv.items[id].val,
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
                if player_inv.items[id].val == 0 {
                    player_inv.items[id] = *item;
                    item.val = 0;
                    save_item_list.push(id);
                    break;
                }
            }
        }
    }

    for slot in save_item_list.iter() {
        save_inv_item(world, storage, entity, *slot).await?;
    }

    Ok(())
}

pub async fn check_inv_item_space(
    world: &GameWorld,
    entity: &crate::GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<bool> {
    let mut total_left = if item.val == 0 { 1 } else { item.val };
    let lock = world.read().await;
    let player_inv = lock.get::<&Inventory>(entity.0)?;
    let mut empty_space_count = 0;

    //First try to add it to other of the same type
    for id in 0..MAX_INV {
        if base.stackable
            && player_inv.items[id].num == item.num
            && player_inv.items[id].val < base.stacklimit
            && player_inv.items[id].val > 0
        {
            if player_inv.items[id].val + total_left > base.stacklimit {
                total_left = total_left + player_inv.items[id].val - base.stacklimit;
            } else {
                return Ok(true);
            }
        } else if player_inv.items[id].val == 0 {
            if !base.stackable {
                return Ok(true);
            }

            empty_space_count += 1;
        }
    }

    Ok(empty_space_count > 0)
}

pub async fn check_inv_item_partial_space(
    world: &GameWorld,
    entity: &crate::GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<(u16, u16)> {
    let mut total_left = if item.val == 0 { 1 } else { item.val };
    let start_val = if item.val == 0 { 1 } else { item.val };
    let lock = world.read().await;
    let player_inv = lock.get::<&Inventory>(entity.0)?;

    //First try to add it to other of the same type
    if base.stackable {
        for id in 0..MAX_INV {
            if player_inv.items[id].num == item.num
                && player_inv.items[id].val < base.stacklimit
                && player_inv.items[id].val > 0
            {
                if player_inv.items[id].val + total_left > base.stacklimit {
                    total_left = total_left + player_inv.items[id].val - base.stacklimit;
                } else {
                    return Ok((0, start_val));
                }
            }
        }
    }

    for id in 0..MAX_INV {
        if player_inv.items[id].val == 0 {
            return Ok((0, start_val));
        }
    }

    Ok((total_left, start_val))
}

#[allow(clippy::too_many_arguments)]
#[inline]
pub async fn set_inv_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: &mut Item,
    base: &ItemData,
    slot: usize,
    amount: u16,
) -> Result<u16> {
    let player_inv = world.cloned_get_or_err::<Inventory>(entity).await?;

    let mut rem = 0u16;
    let item_min = std::cmp::min(amount, item.val);

    if player_inv.items[slot].val == 0 {
        {
            let lock = world.write().await;
            let mut inv = lock.get::<&mut Inventory>(entity.0)?;
            inv.items[slot] = *item;
            inv.items[slot].val = item_min;
        }

        save_inv_item(world, storage, entity, slot).await?;
        return Ok(0);
    } else if player_inv.items[slot].num == item.num {
        {
            let lock = world.write().await;
            let mut inv = lock.get::<&mut Inventory>(entity.0)?;
            rem = val_add_amount_rem(
                &mut inv.items[slot].val,
                &mut item.val,
                item_min,
                base.stacklimit,
            );
        }

        save_inv_item(world, storage, entity, slot).await?;
    }

    Ok(rem)
}

#[inline]
pub async fn give_inv_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: &mut Item,
) -> Result<()> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_inv_item(world, storage, entity, item, base).await
}

pub async fn check_inv_space(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: &mut Item,
) -> Result<bool> {
    let base = &storage.bases.items[item.num as usize];

    check_inv_item_space(world, entity, item, base).await
}

//checks if we only got partial or not if so returns how many we got.
//Returns is_less, amount removed, amount it started with.
pub async fn check_inv_partial_space(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: &mut Item,
) -> Result<(bool, u16, u16)> {
    let base = &storage.bases.items[item.num as usize];

    let (left, start) = check_inv_item_partial_space(world, entity, item, base).await?;

    if left < start {
        Ok((true, start - left, start))
    } else {
        Ok((false, start, 0))
    }
}

#[inline]
pub async fn set_inv_slot(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: &mut Item,
    slot: usize,
    amount: u16,
) -> Result<u16> {
    let base = &storage.bases.items[item.num as usize];

    set_inv_item(world, storage, entity, item, base, slot, amount).await
}

#[inline]
pub async fn take_inv_items(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    num: u32,
    mut amount: u16,
) -> Result<u16> {
    if count_inv_item(
        num,
        &world.cloned_get_or_err::<Inventory>(entity).await?.items,
    ) >= amount as u64
    {
        while let Some(slot) = find_inv_item(
            num,
            &world.cloned_get_or_err::<Inventory>(entity).await?.items,
        ) {
            let mut take_amount = 0;
            {
                let lock = world.write().await;
                let inv_item = lock.get::<&mut Inventory>(entity.0);
                if let Ok(mut invitem) = inv_item {
                    take_amount = invitem.items[slot].val;
                    invitem.items[slot].val = invitem.items[slot].val.saturating_sub(amount);
                }
            }
            amount = amount.saturating_sub(take_amount);

            save_inv_item(world, storage, entity, slot).await?;

            if amount == 0 {
                return Ok(0);
            }
        }
    }

    Ok(amount)
}

#[inline]
pub async fn take_inv_itemslot(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    slot: usize,
    mut amount: u16,
) -> Result<u16> {
    let player_inv = world.cloned_get_or_err::<Inventory>(entity).await?;
    amount = std::cmp::min(amount, player_inv.items[slot].val);
    {
        let lock = world.write().await;
        let inv_item = lock.get::<&mut Inventory>(entity.0);
        if let Ok(mut player_inv) = inv_item {
            player_inv.items[slot].val = player_inv.items[slot].val.saturating_sub(amount);
            if player_inv.items[slot].val == 0 {
                player_inv.items[slot] = Item::default();
            }
        }
    }
    save_inv_item(world, storage, entity, slot).await?;

    let lock = world.read().await;
    let val = lock.get::<&Inventory>(entity.0)?.items[slot].val;
    Ok(val)
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
pub async fn auto_set_trade_item(
    world: &GameWorld,
    entity: &crate::GlobalKey,
    item: &mut Item,
    base: &ItemData,
) -> Result<Vec<usize>> {
    let mut save_slot_list = Vec::new();

    {
        let lock = world.write().await;
        let mut player_trade = lock.get::<&mut TradeItem>(entity.0)?;
        while let Some(slot) = find_trade_slot(item, &player_trade.items, base) {
            if player_trade.items[slot].val == 0 {
                player_trade.items[slot] = *item;
                item.val = 0;
                save_slot_list.push(slot);
                break;
            }

            let rem = val_add_rem(
                &mut player_trade.items[slot].val,
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
pub async fn give_trade_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: &mut Item,
) -> Result<Vec<usize>> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_trade_item(world, entity, item, base).await
}

pub async fn check_temp_inv_space(
    storage: &GameStore,
    item: &mut Item,
    temp_inv: &mut Inventory,
) -> Result<bool> {
    let base = &storage.bases.items[item.num as usize];

    check_temp_inv_item_space(item, base, temp_inv).await
}

pub async fn check_temp_inv_item_space(
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
pub async fn auto_set_temp_inv_item(
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
pub async fn give_temp_inv_item(
    storage: &GameStore,
    item: &mut Item,
    temp_inv: &mut Inventory,
) -> Result<()> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_temp_inv_item(item, base, temp_inv).await
}

pub async fn player_unequip(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    slot: usize,
) -> Result<bool> {
    let has_none = {
        let lock = world.read().await;
        let equipment = lock.get::<&Equipment>(entity.0)?;
        equipment.items[slot].val == 0
    };

    if has_none {
        return Ok(true);
    }

    let mut item = {
        let lock = world.read().await;
        let equipment = lock.get::<&Equipment>(entity.0)?;
        let item = equipment.items[slot];
        item
    };

    if !check_inv_space(world, storage, entity, &mut item).await? {
        return Ok(false);
    }

    give_inv_item(world, storage, entity, &mut item).await?;

    {
        let lock = world.write().await;
        lock.get::<&mut Equipment>(entity.0)?.items[slot] = Item::default();
    }

    storage
        .sql_request
        .send(SqlRequests::Equipment((*entity, slot)))
        .await?;
    send_equipment(world, storage, entity).await?;

    Ok(true)
}

pub async fn player_equip(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    item: Item,
    slot: usize,
) -> Result<()> {
    {
        let lock = world.write().await;
        lock.get::<&mut Equipment>(entity.0)?.items[slot] = item;
    }
    storage
        .sql_request
        .send(SqlRequests::Equipment((*entity, slot)))
        .await?;
    send_equipment(world, storage, entity).await?;

    Ok(())
}

pub async fn player_use_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    slot: u16,
) -> Result<()> {
    if slot as usize >= MAX_INV {
        return Ok(());
    }

    let item = {
        let lock = world.read().await;
        let item = lock.get::<&Inventory>(entity.0)?.items[slot as usize];
        item
    };

    if item.val == 0 {
        return Ok(());
    }

    let base = &storage.bases.items[item.num as usize];

    match base.itemtype {
        ItemTypes::Consume => {
            if base.data[0] > 0 {
                let player_vital = world.get_or_err::<Vitals>(entity).await?;
                let set_vital = player_vital.vital[VitalTypes::Hp as usize]
                    .saturating_add(base.data[0] as i32)
                    .min(player_vital.vitalmax[VitalTypes::Hp as usize]);
                player_set_vital(world, storage, entity, VitalTypes::Hp, set_vital).await?;

                DataTaskToken::Damage(world.get_or_default::<Position>(entity).await.map)
                    .add_task(
                        storage,
                        damage_packet(
                            *entity,
                            base.data[0] as u16,
                            world.get_or_default::<Position>(entity).await,
                            false,
                        )?,
                    )
                    .await?;
            }

            if base.data[1] > 0 {
                let player_vital = world.get_or_err::<Vitals>(entity).await?;
                let set_vital = player_vital.vital[VitalTypes::Mp as usize]
                    .saturating_add(base.data[1] as i32)
                    .min(player_vital.vitalmax[VitalTypes::Mp as usize]);
                player_set_vital(world, storage, entity, VitalTypes::Mp, set_vital).await?;
            }

            if base.data[2] > 0 {
                let player_vital = world.get_or_err::<Vitals>(entity).await?;
                let set_vital = player_vital.vital[VitalTypes::Sp as usize]
                    .saturating_add(base.data[2] as i32)
                    .min(player_vital.vitalmax[VitalTypes::Sp as usize]);
                player_set_vital(world, storage, entity, VitalTypes::Sp, set_vital).await?;
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

            if !player_unequip(world, storage, entity, eqslot).await? {
                // ToDo Warning cannot unequip
                return Ok(());
            }
            player_equip(world, storage, entity, item, eqslot).await?;
        }
        _ => return Ok(()),
    }

    if let Some(_sfx) = &base.sound_index {
        send_playitemsfx(world, storage, entity, item.num as u16).await?;
    }

    take_inv_itemslot(world, storage, entity, slot as usize, 1).await?;

    Ok(())
}
