use hecs::World;

use crate::{containers::*, gameloop::*, gametypes::*, items::*, players::*, sql::*};

#[inline]
pub fn save_item(world: &mut World, storage: &Storage, entity: &Entity, slot: usize) {
    let _ = update_inv(storage, world, entity, slot);
    let _ = send_invslot(world, storage, entity, slot);
}

#[inline]
pub fn get_inv_scope(invtype: InvType) -> (usize, usize) {
    match invtype {
        InvType::Normal => (0, 36),
        InvType::Key => (36, 72),
        InvType::Quest => (72, 108),
        InvType::Script => (108, 378),
    }
}

#[inline]
pub fn get_inv_type(slot: usize) -> InvType {
    match slot {
        0..=35 => InvType::Normal,
        36..=71 => InvType::Key,
        72..=107 => InvType::Quest,
        _ => InvType::Script,
    }
}

#[inline]
pub fn get_inv_itemtype(base: &ItemData) -> InvType {
    match base.itemtype {
        ItemTypes::Questitem => InvType::Quest,
        ItemTypes::Key => InvType::Key,
        _ => InvType::Normal,
    }
}

#[inline]
pub fn count_inv_item(num: u32, inv: &[Item], scope: (usize, usize)) -> u64 {
    (scope.0..scope.1)
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
pub fn find_inv_item(num: u32, inv: &[Item], scope: &(usize, usize)) -> Option<usize> {
    (scope.0..scope.1).find(|id| inv[*id].num == num && inv[*id].val > 0)
}

#[inline]
pub fn find_slot(
    item: &Item,
    inv: &[Item],
    base: &ItemData,
    scope: &(usize, usize),
) -> Option<usize> {
    if base.stackable {
        if let Some(id) = (scope.0..scope.1).find(|id| {
            inv[*id].num == item.num && inv[*id].val < base.stacklimit && inv[*id].val > 0
        }) {
            return Some(id);
        }
    }

    (scope.0..scope.1).find(|id| inv[*id].val == 0)
}

#[inline]
pub fn auto_set_inv_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
    invtype: InvType,
) -> u16 {
    let player_inv = world.cloned_get_or_panic::<Inventory>(entity);

    let mut rem = 0u16;
    let scope = get_inv_scope(invtype);

    while let Some(slot) = find_slot(item, &player_inv.items, base, &scope) {
        if player_inv.items[slot].val == 0 {
            {
                world
                    .get::<&mut Inventory>(entity.0)
                    .expect("Could not find Inventory")
                    .items[slot] = *item;
            }
            item.val = 0;
            save_item(world, storage, entity, slot);
            break;
        }

        let mut playerinv_val = player_inv.items[slot].val;
        rem = val_add_rem(&mut playerinv_val, &mut item.val, base.stacklimit);
        save_item(world, storage, entity, slot);

        if rem == 0 {
            break;
        }
    }

    rem
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
    invtype: InvType,
) -> u16 {
    let player_inv = world.cloned_get_or_panic::<Inventory>(entity);

    let mut rem = 0u16;
    let mut item_min = std::cmp::min(amount, item.val);

    if player_inv.items[slot].val == 0 {
        {
            world
                .get::<&mut Inventory>(entity.0)
                .expect("Could not find Inventory")
                .items[slot] = *item;
            world
                .get::<&mut Inventory>(entity.0)
                .expect("Could not find Inventory")
                .items[slot]
                .val = item_min;
        }
        item.val = item.val.saturating_sub(item_min);
        save_item(world, storage, entity, slot);
        return 0;
    }

    if player_inv.items[slot].num == item.num {
        let mut playerinv_val = player_inv.items[slot].val;
        item_min = val_add_amount_rem(&mut playerinv_val, &mut item.val, item_min, base.stacklimit);

        save_item(world, storage, entity, slot);

        if item_min > 0 {
            let mut itemtemp = *item;
            itemtemp.val = item_min;

            rem = auto_set_inv_item(world, storage, entity, &mut itemtemp, base, invtype);

            if rem < item_min {
                item.val = item.val.saturating_sub(item_min.saturating_sub(rem));
            }
        }
    }

    rem
}

#[inline]
pub fn give_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
) -> u16 {
    let base = &storage.bases.items[item.num as usize];
    let invtype = get_inv_itemtype(base);

    auto_set_inv_item(world, storage, entity, item, base, invtype)
}

#[inline]
pub fn set_inv_slot(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
    slot: usize,
    amount: u16,
) -> Option<u16> {
    let base = &storage.bases.items[item.num as usize];
    let invtype = get_inv_itemtype(base);

    if get_inv_type(slot) != invtype {
        return None;
    }

    Some(set_inv_item(
        world, storage, entity, item, base, slot, amount, invtype,
    ))
}

#[inline]
pub fn take_inv_items(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    num: u32,
    mut amount: u16,
    invtype: InvType,
) -> u16 {
    let player_inv = world.cloned_get_or_panic::<Inventory>(entity);

    let scope = get_inv_scope(invtype);

    if count_inv_item(num, &player_inv.items, scope) >= amount as u64 {
        while let Some(slot) = find_inv_item(num, &player_inv.items, &scope) {
            {
                world
                    .get::<&mut Inventory>(entity.0)
                    .expect("Could not find Inventory")
                    .items[slot]
                    .val = player_inv.items[slot].val.saturating_sub(amount);
            }
            amount = player_inv.items[slot].val;

            save_item(world, storage, entity, slot);

            if amount == 0 {
                return 0;
            }
        }
    }

    amount
}

#[inline]
pub fn take_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    num: u32,
    amount: u16,
) -> u16 {
    let base = &storage.bases.items[num as usize];
    let invtype = get_inv_itemtype(base);

    take_inv_items(world, storage, entity, num, amount, invtype)
}

#[inline]
pub fn take_itemslot(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    slot: usize,
    mut amount: u16,
) -> u16 {
    let player_inv = world.cloned_get_or_panic::<Inventory>(entity);

    amount = std::cmp::min(amount, player_inv.items[slot].val);
    {
        world
            .get::<&mut Inventory>(entity.0)
            .expect("Could not find Inventory")
            .items[slot]
            .val = player_inv.items[slot].val.saturating_sub(amount);
    }
    save_item(world, storage, entity, slot);

    player_inv.items[slot].val
}
