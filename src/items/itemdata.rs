use crate::gametypes::{ItemTypes, Rgba};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, Derivative)]
#[derivative(Default(new = "true"))]
pub struct ItemData {
    pub name: String,
    pub levelreq: u16,
    pub soundid: u16,
    pub sprite: u16,
    pub animation: Option<u32>,
    pub data: [i16; 20],
    pub itemtype: ItemTypes,
    pub itemtype2: u8,
    pub breakable: bool,
    pub stackable: bool,
    #[derivative(Default(value = "1"))]
    pub stacklimit: u16,
    pub baseprice: u64,
    pub repairable: bool,
    pub rgba: Rgba,
}
