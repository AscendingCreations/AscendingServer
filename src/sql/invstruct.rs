use crate::sql::integers::Shifting;
use crate::{gametypes::*, items::Item};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct PGInvItem {
    uid: i64,
    id: i16,
    num: i32,
    val: i16,
    itemlevel: i16,
    data: Vec<i16>,
}

impl PGInvItem {
    pub fn new(inv: &[Item], uid: i64) -> Vec<PGInvItem> {
        let mut items: Vec<PGInvItem> = Vec::with_capacity(MAX_INV);

        for (id, invitem) in inv.iter().enumerate() {
            items.push(PGInvItem {
                uid,
                id: i16::unshift_signed(&(id as u16)),
                num: i32::unshift_signed(&invitem.num),
                val: i16::unshift_signed(&invitem.val),
                itemlevel: i16::unshift_signed(&(invitem.level as u16)),
                data: invitem.data.to_vec(),
            });
        }

        items
    }

    pub fn single(inv: &[Item], uid: i64, slot: usize) -> PGInvItem {
        PGInvItem {
            uid,
            id: i16::unshift_signed(&(slot as u16)),
            num: i32::unshift_signed(&inv[slot].num),
            val: i16::unshift_signed(&inv[slot].val),
            itemlevel: i16::unshift_signed(&(inv[slot].level as u16)),
            data: inv[slot].data.to_vec(),
        }
    }

    pub fn into_item(self, inv: &mut [Item]) {
        let slot = self.id as usize;

        inv[slot].num = self.num.shift_signed();
        inv[slot].val = self.val.shift_signed();
        inv[slot].level = self.itemlevel.shift_signed() as u8;
        inv[slot].data = self.data[..5].try_into().unwrap_or([0; 5]);
    }

    pub fn array_into_items(items: Vec<PGInvItem>, inv: &mut [Item]) {
        for slot in items {
            slot.into_item(inv);
        }
    }
}
