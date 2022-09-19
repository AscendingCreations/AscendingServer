use crate::{items::Item, sql};
use std::convert::TryInto;

#[derive(Debug, Queryable, Insertable, Identifiable, AsChangeset)]
#[diesel(table_name = sql::equipment)]
#[diesel(primary_key(uid, id))]
pub struct PGEquipItem {
    uid: i64,
    id: i16,
    num: i32,
    val: i16,
    itemlevel: i16,
    data: Vec<i16>,
}

impl PGEquipItem {
    pub fn new(inv: &[Item], uid: i64) -> Vec<PGEquipItem> {
        let mut items: Vec<PGEquipItem> = Vec::with_capacity(37);

        for (id, invitem) in inv.iter().enumerate() {
            items.push(PGEquipItem {
                uid,
                id: id as i16,
                num: invitem.num as i32,
                val: invitem.val as i16,
                itemlevel: invitem.level as i16,
                data: invitem.data.to_vec(),
            });
        }

        items
    }

    pub fn single(inv: &[Item], uid: i64, slot: usize) -> PGEquipItem {
        PGEquipItem {
            uid,
            id: slot as i16,
            num: inv[slot].num as i32,
            val: inv[slot].val as i16,
            itemlevel: inv[slot].level as i16,
            data: inv[slot].data.to_vec(),
        }
    }

    pub fn into_item(self, inv: &mut [Item]) {
        let slot = self.id as usize;

        inv[slot].num = self.num as u32;
        inv[slot].val = self.val as u16;
        inv[slot].level = self.itemlevel as u8;
        inv[slot].data = self.data[..5].try_into().unwrap_or([0; 5]);
    }

    pub fn array_into_items(items: Vec<PGEquipItem>, inv: &mut [Item]) {
        for slot in items {
            slot.into_item(inv);
        }
    }
}