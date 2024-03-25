use hecs::World;

use crate::{containers::*, gameloop::*, gametypes::*, items::*, players::*, sql::*};

#[inline]
pub fn save_storage_item(world: &mut World, storage: &Storage, entity: &Entity, slot: usize) {
    let _ = update_storage(storage, world, entity, slot);
    let _ = send_storageslot(world, storage, entity, slot);
}

#[inline]
pub fn count_storage_item(num: u32, storage: &[Item]) -> u64 {
    (0..MAX_STORAGE)
        .filter_map(|id| {
            if storage[id].num == num && storage[id].val > 0 {
                Some(storage[id].val as u64)
            } else {
                None
            }
        })
        .fold(0u64, u64::saturating_add)
}

#[inline]
pub fn find_storage_item(num: u32, storage: &[Item]) -> Option<usize> {
    (0..MAX_STORAGE).find(|id| storage[*id].num == num && storage[*id].val > 0)
}

#[inline]
pub fn find_storage_slot(item: &Item, storage: &[Item], base: &ItemData) -> Option<usize> {
    if base.stackable {
        if let Some(id) = (0..MAX_STORAGE).find(|id| {
            storage[*id].num == item.num
                && storage[*id].val < base.stacklimit
                && storage[*id].val > 0
        }) {
            return Some(id);
        }
    }

    (0..MAX_STORAGE).find(|id| storage[*id].val == 0)
}

#[inline]
pub fn auto_set_storage_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
) -> u16 {
    let mut rem = 0u16;
    let mut save_item_list = Vec::new();

    {
        let mut player_storage = world
            .get::<&mut PlayerStorage>(entity.0)
            .expect("Could not find PlayerStorage");
        while let Some(slot) = find_storage_slot(item, &player_storage.items, base) {
            if player_storage.items[slot].val == 0 {
                player_storage.items[slot] = *item;
                item.val = 0;
                save_item_list.push(slot);
                break;
            }

            rem = val_add_rem(
                &mut player_storage.items[slot].val,
                &mut item.val,
                base.stacklimit,
            );
            save_item_list.push(slot);

            if rem == 0 {
                break;
            }
        }
    }

    save_item_list.iter().for_each(|slot| {
        save_storage_item(world, storage, entity, *slot);
    });

    rem
}

#[allow(clippy::too_many_arguments)]
#[inline]
pub fn set_storage_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
    slot: usize,
    amount: u16,
) -> u16 {
    let player_storage = world.cloned_get_or_panic::<PlayerStorage>(entity);

    let mut rem = 0u16;
    let mut item_min = std::cmp::min(amount, item.val);

    if player_storage.items[slot].val == 0 {
        {
            world
                .get::<&mut PlayerStorage>(entity.0)
                .expect("Could not find PlayerStorage")
                .items[slot] = *item;
            world
                .get::<&mut PlayerStorage>(entity.0)
                .expect("Could not find PlayerStorage")
                .items[slot]
                .val = item_min;
        }
        item.val = item.val.saturating_sub(item_min);
        save_storage_item(world, storage, entity, slot);
        return 0;
    }

    if player_storage.items[slot].num == item.num {
        let mut playerstorage_val = player_storage.items[slot].val;
        item_min = val_add_amount_rem(
            &mut playerstorage_val,
            &mut item.val,
            item_min,
            base.stacklimit,
        );

        save_storage_item(world, storage, entity, slot);

        if item_min > 0 {
            let mut itemtemp = *item;
            itemtemp.val = item_min;

            rem = auto_set_storage_item(world, storage, entity, &mut itemtemp, base);

            if rem < item_min {
                item.val = item.val.saturating_sub(item_min.saturating_sub(rem));
            }
        }
    }

    rem
}

#[inline]
pub fn give_storage_item(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
) -> u16 {
    let base = &storage.bases.items[item.num as usize];

    auto_set_storage_item(world, storage, entity, item, base)
}

#[inline]
pub fn set_storage_slot(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    item: &mut Item,
    slot: usize,
    amount: u16,
) -> Option<u16> {
    let base = &storage.bases.items[item.num as usize];

    Some(set_storage_item(
        world, storage, entity, item, base, slot, amount,
    ))
}

#[inline]
pub fn take_storage_items(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    num: u32,
    mut amount: u16,
) -> u16 {
    let player_storage = world.cloned_get_or_panic::<PlayerStorage>(entity);

    if count_storage_item(num, &player_storage.items) >= amount as u64 {
        while let Some(slot) = find_storage_item(num, &player_storage.items) {
            {
                world
                    .get::<&mut PlayerStorage>(entity.0)
                    .expect("Could not find PlayerStorage")
                    .items[slot]
                    .val = player_storage.items[slot].val.saturating_sub(amount);
            }
            amount = player_storage.items[slot].val;

            save_storage_item(world, storage, entity, slot);

            if amount == 0 {
                return 0;
            }
        }
    }

    amount
}

#[inline]
pub fn take_storage_itemslot(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    slot: usize,
    mut amount: u16,
) -> u16 {
    let player_storage = world.cloned_get_or_panic::<PlayerStorage>(entity);
    amount = std::cmp::min(amount, player_storage.items[slot].val);
    {
        if let Ok(mut player_storage) = world.get::<&mut PlayerStorage>(entity.0) {
            player_storage.items[slot].val = player_storage.items[slot].val.saturating_sub(amount);
            if player_storage.items[slot].val == 0 {
                player_storage.items[slot] = Item::default();
            }
        }
    }
    save_storage_item(world, storage, entity, slot);

    world.cloned_get_or_panic::<PlayerStorage>(entity).items[slot].val
}
