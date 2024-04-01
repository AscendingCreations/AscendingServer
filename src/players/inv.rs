use hecs::World;

use crate::{containers::*, gametypes::*, items::*, players::*, socket::*, sql::*};

#[inline]
pub fn save_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
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
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
) -> Result<u16> {
    let mut rem = 0u16;
    let mut save_item_list = Vec::new();

    {
        let mut player_inv = world.get::<&mut Inventory>(entity.0)?;
        while let Some(slot) = find_inv_slot(item, &player_inv.items, base) {
            if player_inv.items[slot].val == 0 {
                player_inv.items[slot] = *item;
                item.val = 0;
                save_item_list.push(slot);
                break;
            }

            rem = val_add_rem(
                &mut player_inv.items[slot].val,
                &mut item.val,
                base.stacklimit,
            );
            save_item_list.push(slot);

            if rem == 0 {
                break;
            }
        }
    }

    for slot in save_item_list.iter() {
        save_inv_item(world, storage, entity, *slot)?;
    }

    Ok(rem)
}

#[allow(clippy::too_many_arguments)]
#[inline]
pub fn set_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
    slot: usize,
    amount: u16,
) -> Result<u16> {
    let player_inv = world.cloned_get_or_err::<Inventory>(entity)?;

    let mut rem = 0u16;
    let item_min = std::cmp::min(amount, item.val);

    if player_inv.items[slot].val == 0 {
        {
            let mut inv = world.get::<&mut Inventory>(entity.0)?;
            inv.items[slot] = *item;
            inv.items[slot].val = item_min;
        }

        save_inv_item(world, storage, entity, slot)?;
        return Ok(0);
    } else if player_inv.items[slot].num == item.num {
        {
            rem = val_add_amount_rem(
                &mut world.get::<&mut Inventory>(entity.0)?.items[slot].val,
                &mut item.val,
                item_min,
                base.stacklimit,
            );
        }

        save_inv_item(world, storage, entity, slot)?;
    }

    Ok(rem)
}

#[inline]
pub fn give_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
) -> Result<u16> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_inv_item(world, storage, entity, item, base)
}

#[inline]
pub fn set_inv_slot(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
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
    entity: &crate::Entity,
    num: u32,
    mut amount: u16,
) -> Result<u16> {
    let player_inv = world.cloned_get_or_err::<Inventory>(entity)?;

    if count_inv_item(num, &player_inv.items) >= amount as u64 {
        while let Some(slot) = find_inv_item(num, &player_inv.items) {
            {
                world.get::<&mut Inventory>(entity.0)?.items[slot].val =
                    player_inv.items[slot].val.saturating_sub(amount);
            }
            amount = player_inv.items[slot].val;

            save_inv_item(world, storage, entity, slot)?;

            if amount == 0 {
                return Ok(0);
            }
        }
    }

    Ok(amount)
}

#[inline]
pub fn take_inv_itemslot(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    slot: usize,
    mut amount: u16,
) -> Result<u16> {
    let player_inv = world.cloned_get_or_err::<Inventory>(entity)?;
    amount = std::cmp::min(amount, player_inv.items[slot].val);
    {
        if let Ok(mut player_inv) = world.get::<&mut Inventory>(entity.0) {
            player_inv.items[slot].val = player_inv.items[slot].val.saturating_sub(amount);
            if player_inv.items[slot].val == 0 {
                player_inv.items[slot] = Item::default();
            }
        }
    }
    save_inv_item(world, storage, entity, slot)?;

    Ok(world.get::<&Inventory>(entity.0)?.items[slot].val)
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
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
) -> Result<Vec<usize>> {
    let mut save_slot_list = Vec::new();

    {
        let mut player_trade = world.get::<&mut TradeItem>(entity.0)?;
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
pub fn give_trade_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
) -> Result<Vec<usize>> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_trade_item(world, entity, item, base)
}
