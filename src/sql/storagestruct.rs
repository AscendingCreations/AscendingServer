use crate::sql::integers::Shifting;
use crate::{gametypes::*, items::Item};
use itertools::Itertools;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct PGStorageItem {
    uid: i64,
    id: i16,
    num: i32,
    val: i16,
    itemlevel: i16,
    data: Vec<i16>,
}

impl PGStorageItem {
    pub fn new(storage_slot: &[Item], uid: i64) -> Vec<PGStorageItem> {
        let mut items: Vec<PGStorageItem> = Vec::with_capacity(MAX_STORAGE);

        for (id, storageitem) in storage_slot.iter().enumerate() {
            items.push(PGStorageItem {
                uid,
                id: id as i16,
                num: i32::unshift_signed(&storageitem.num),
                val: i16::unshift_signed(&storageitem.val),
                itemlevel: i16::unshift_signed(&(storageitem.level as u16)),
                data: storageitem.data.to_vec(),
            });
        }

        items
    }

    pub fn single(storage_slot: &[Item], uid: i64, slot: usize) -> PGStorageItem {
        PGStorageItem {
            uid,
            id: slot as i16,
            num: i32::unshift_signed(&storage_slot[slot].num),
            val: i16::unshift_signed(&storage_slot[slot].val),
            itemlevel: i16::unshift_signed(&(storage_slot[slot].level as u16)),
            data: storage_slot[slot].data.to_vec(),
        }
    }

    pub fn into_item(self, storage_slot: &mut [Item]) {
        let slot = self.id as usize;

        storage_slot[slot].num = self.num.shift_signed();
        storage_slot[slot].val = self.val.shift_signed();
        storage_slot[slot].level = self.itemlevel.shift_signed() as u8;
        storage_slot[slot].data = self.data[..5].try_into().unwrap_or([0; 5]);
    }

    pub fn array_into_items(items: Vec<PGStorageItem>, storage_slot: &mut [Item]) {
        for slot in items {
            slot.into_item(storage_slot);
        }
    }

    pub fn into_insert_all(items: Vec<PGStorageItem>) -> Vec<String> {
        let mut vec = Vec::with_capacity(items.len());

        for item in items {
            vec.push(item.into_insert())
        }

        vec
    }

    fn into_insert(self) -> String {
        let data = self
            .data
            .iter()
            .format_with(", ", |elt, f| f(&format_args!("{}", elt)));

        format!(
            r#"
        INSERT INTO public.storage(
            uid, id, num, val, itemlevel, data)
            VALUES ({0}, {1}, {2}, {3}, {4}, '{{{5}}}');
        "#,
            self.uid, self.id, self.num, self.val, self.itemlevel, data
        )
    }

    pub fn into_update_all(items: Vec<PGStorageItem>) -> Vec<String> {
        let mut vec = Vec::with_capacity(items.len());

        for item in items {
            vec.push(item.into_update())
        }

        vec
    }

    pub fn into_update(self) -> String {
        let data = self
            .data
            .iter()
            .format_with(", ", |elt, f| f(&format_args!("{}", elt)));

        format!(
            r#"
            UPDATE public.storage
	        SET num={0}, val={1}, itemlevel={2}, data='{{{3}}}'
	        WHERE uid = {4} and id = {5};
        "#,
            self.num, self.val, self.itemlevel, data, self.uid, self.id
        )
    }
}
