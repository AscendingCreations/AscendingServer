use crate::{containers::IndexMap, gametypes::*, items::*, maps::*, npcs::*};
pub struct Bases {
    pub maps: IndexMap<MapPosition, Map>,
    pub npcs: Vec<NpcData>,
    pub items: Vec<ItemData>,
    pub shops: Vec<ShopData>,
}

impl Bases {
    pub fn new() -> Option<Self> {
        Some(Self {
            maps: IndexMap::default(),
            npcs: vec![NpcData::default(); MAX_NPCS],
            items: vec![ItemData::default(); MAX_ITEMS],
            shops: vec![ShopData::default(); MAX_SHOPS],
        })
    }
}
