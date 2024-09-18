use crate::gametypes::MAX_SHOP_ITEM;
use educe::Educe;
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use std::fs::OpenOptions;
use std::io::Read;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Educe, Readable, Writable)]
#[educe(Default)]
pub struct ShopItem {
    pub index: u16,
    pub amount: u16,
    pub price: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, Educe, Readable, Writable)]
#[educe(Default)]
pub struct ShopData {
    pub name: String,
    pub max_item: u16,
    pub item: [ShopItem; MAX_SHOP_ITEM],
}

pub fn load_shops() -> Vec<ShopData> {
    let mut shop_data: Vec<ShopData> = Vec::new();

    let mut count = 0;
    let mut got_data = true;

    while got_data {
        if let Some(data) = load_shop(count) {
            shop_data.push(data);
            count += 1;
            got_data = true;
        } else {
            got_data = false;
        }
    }

    shop_data
}

fn load_shop(id: usize) -> Option<ShopData> {
    let name = format!("./data/shops/{}.bin", id);

    match OpenOptions::new().read(true).open(name) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            match file.read_to_end(&mut bytes) {
                Ok(_) => Some(ShopData::read_from_buffer(&bytes).unwrap()),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}
