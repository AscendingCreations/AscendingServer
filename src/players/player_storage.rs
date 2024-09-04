use crate::{containers::*, gametypes::*, items::*, players::*, socket::*, sql::*};

#[inline]
pub async fn save_storage_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    slot: usize,
) -> Result<()> {
    storage
        .sql_request
        .send(SqlRequests::Storage((*entity, slot)))
        .await?;
    //update_storage(storage, world, entity, slot).await?;
    send_storageslot(world, storage, entity, slot).await
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
//This should always be successful unless an error occurs since we check for space ahead of time.
pub async fn auto_set_storage_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
) -> Result<()> {
    let mut save_item_list = Vec::new();
    let mut total_left = if item.val == 0 { 1 } else { item.val };

    {
        let lock = world.write().await;
        let mut player_storage = lock.get::<&mut PlayerStorage>(entity.0)?;

        if base.stackable {
            for id in 0..MAX_STORAGE {
                if player_storage.items[id].num == item.num
                    && player_storage.items[id].val < base.stacklimit
                    && player_storage.items[id].val > 0
                {
                    val_add_rem(
                        &mut player_storage.items[id].val,
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

        if total_left > 0 {
            for id in 0..MAX_STORAGE {
                if player_storage.items[id].val == 0 {
                    player_storage.items[id] = *item;
                    item.val = 0;
                    save_item_list.push(id);
                    break;
                }
            }
        }
    }

    for slot in save_item_list.iter() {
        save_storage_item(world, storage, entity, *slot).await?;
    }

    Ok(())
}

pub async fn check_storage_space(
    world: &GameWorld,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
) -> Result<bool> {
    let mut total_left = if item.val == 0 { 1 } else { item.val };
    let mut empty_space_count = 0;
    let lock = world.read().await;
    let player_storage = lock.get::<&PlayerStorage>(entity.0)?;

    //First try to add it to other of the same type
    for id in 0..MAX_STORAGE {
        if base.stackable
            && player_storage.items[id].num == item.num
            && player_storage.items[id].val < base.stacklimit
            && player_storage.items[id].val > 0
        {
            if player_storage.items[id].val + total_left > base.stacklimit {
                total_left = total_left + player_storage.items[id].val - base.stacklimit;
            } else {
                return Ok(true);
            }
        } else if player_storage.items[id].val == 0 {
            if !base.stackable {
                return Ok(true);
            }

            empty_space_count += 1;
        }
    }

    Ok(empty_space_count > 0)
}

pub async fn check_storage_item_partial_space(
    world: &GameWorld,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
) -> Result<(u16, u16)> {
    let mut total_left = if item.val == 0 { 1 } else { item.val };
    let start_val = if item.val == 0 { 1 } else { item.val };
    let lock = world.read().await;
    let player_storage = lock.get::<&PlayerStorage>(entity.0)?;

    //First try to add it to other of the same type
    if base.stackable {
        for id in 0..MAX_STORAGE {
            if player_storage.items[id].num == item.num
                && player_storage.items[id].val < base.stacklimit
                && player_storage.items[id].val > 0
            {
                if player_storage.items[id].val + total_left > base.stacklimit {
                    total_left = total_left + player_storage.items[id].val - base.stacklimit;
                } else {
                    return Ok((0, start_val));
                }
            }
        }
    }

    for id in 0..MAX_STORAGE {
        if player_storage.items[id].val == 0 {
            return Ok((0, start_val));
        }
    }

    Ok((total_left, start_val))
}

//checks if we only got partial or not if so returns how many we got.
//Returns is_less, amount removed, amount it started with.
pub async fn check_storage_partial_space(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    item: &mut Item,
) -> Result<(bool, u16, u16)> {
    let base = &storage.bases.items[item.num as usize];

    let (left, start) = check_storage_item_partial_space(world, entity, item, base).await?;

    if left < start {
        Ok((true, start - left, start))
    } else {
        Ok((false, start, 0))
    }
}

#[allow(clippy::too_many_arguments)]
#[inline]
pub async fn set_storage_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    item: &mut Item,
    base: &ItemData,
    slot: usize,
    amount: u16,
) -> Result<u16> {
    let player_storage = world.cloned_get_or_err::<PlayerStorage>(entity).await?;

    let mut rem = 0u16;
    let item_min = std::cmp::min(amount, item.val);

    if player_storage.items[slot].val == 0 {
        {
            let lock = world.write().await;
            let mut storage = lock.get::<&mut PlayerStorage>(entity.0)?;
            storage.items[slot] = *item;
            storage.items[slot].val = item_min;
        }
        save_storage_item(world, storage, entity, slot).await?;
        return Ok(0);
    } else if player_storage.items[slot].num == item.num {
        {
            let lock = world.write().await;
            let mut store_item = lock.get::<&mut PlayerStorage>(entity.0)?.items[slot];
            rem = val_add_amount_rem(
                &mut store_item.val,
                &mut item.val,
                item_min,
                base.stacklimit,
            );
        }

        save_storage_item(world, storage, entity, slot).await?;
    }

    Ok(rem)
}

#[inline]
pub async fn give_storage_item(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    item: &mut Item,
) -> Result<()> {
    let base = &storage.bases.items[item.num as usize];

    auto_set_storage_item(world, storage, entity, item, base).await
}

#[inline]
pub async fn check_storage_item_fits(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    item: &mut Item,
) -> Result<bool> {
    let base = &storage.bases.items[item.num as usize];

    check_storage_space(world, entity, item, base).await
}

#[inline]
pub async fn set_storage_slot(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    item: &mut Item,
    slot: usize,
    amount: u16,
) -> Result<u16> {
    let base = &storage.bases.items[item.num as usize];

    set_storage_item(world, storage, entity, item, base, slot, amount).await
}

#[inline]
pub async fn take_storage_items(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    num: u32,
    mut amount: u16,
) -> Result<u16> {
    let player_storage = world.cloned_get_or_err::<PlayerStorage>(entity).await?;

    if count_storage_item(num, &player_storage.items) >= amount as u64 {
        while let Some(slot) = find_storage_item(num, &player_storage.items) {
            {
                let lock = world.write().await;
                lock.get::<&mut PlayerStorage>(entity.0)?.items[slot].val =
                    player_storage.items[slot].val.saturating_sub(amount);
            }
            amount = player_storage.items[slot].val;

            save_storage_item(world, storage, entity, slot).await?;

            if amount == 0 {
                return Ok(0);
            }
        }
    }

    Ok(amount)
}

#[inline]
pub async fn take_storage_itemslot(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
    slot: usize,
    mut amount: u16,
) -> Result<u16> {
    let player_storage = world.cloned_get_or_err::<PlayerStorage>(entity).await?;
    amount = std::cmp::min(amount, player_storage.items[slot].val);
    {
        let lock = world.write().await;
        let store = lock.get::<&mut PlayerStorage>(entity.0);
        if let Ok(mut player_storage) = store {
            player_storage.items[slot].val = player_storage.items[slot].val.saturating_sub(amount);
            if player_storage.items[slot].val == 0 {
                player_storage.items[slot] = Item::default();
            }
        }
    }
    save_storage_item(world, storage, entity, slot).await?;

    let lock = world.read().await;
    let val = lock.get::<&PlayerStorage>(entity.0)?.items[slot].val;
    Ok(val)
}
