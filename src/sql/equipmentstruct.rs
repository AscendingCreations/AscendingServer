use crate::items::Item;
use crate::sql::integers::Shifting;
use itertools::Itertools;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
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
                num: i32::unshift_signed(&invitem.num),
                val: i16::unshift_signed(&invitem.val),
                itemlevel: i16::unshift_signed(&(invitem.level as u16)),
                data: invitem.data.to_vec(),
            });
        }

        items
    }

    pub fn single(inv: &[Item], uid: i64, slot: usize) -> PGEquipItem {
        PGEquipItem {
            uid,
            id: slot as i16,
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

    pub fn array_into_items(items: Vec<PGEquipItem>, inv: &mut [Item]) {
        for slot in items {
            slot.into_item(inv);
        }
    }

    pub fn into_insert_all(items: Vec<PGEquipItem>) -> Vec<String> {
        let mut vec = Vec::with_capacity(items.len());

        for item in items {
            vec.push(item.into_insert())
        }

        vec
    }

    pub fn into_insert(self) -> String {
        let data = self
            .data
            .iter()
            .format_with(", ", |elt, f| f(&format_args!("{}", elt)));

        format!(
            r#"
        INSERT INTO public.equipment(
            uid, id, num, val, itemlevel, data)
            VALUES ({0}, {1}, {2}, {3}, {4}, '{{{5}}}');
        "#,
            self.uid, self.id, self.num, self.val, self.itemlevel, data
        )
    }

    pub fn into_update_all(items: Vec<PGEquipItem>) -> Vec<String> {
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
            UPDATE public.equipment
	        SET num={0}, val={1}, itemlevel={2}, data='{{{3}}}'
	        WHERE uid = {4} and id = {5};
        "#,
            self.num, self.val, self.itemlevel, data, self.uid, self.id
        )
    }
}
