use std::sync::Arc;

use crate::{gametypes::*, items::*, maps::*, npcs::*};

#[derive(Debug)]
pub struct Bases {
    pub maps: IndexMap<MapPosition, Arc<Map>>,
    pub npcs: Vec<Arc<NpcData>>,
    pub items: Vec<Arc<ItemData>>,
    pub shops: Vec<Arc<ShopData>>,
}

impl Bases {
    pub fn new() -> Option<Self> {
        Some(Self {
            maps: IndexMap::default(),
            npcs: vec![Arc::new(NpcData::default()); MAX_NPCS],
            items: vec![Arc::new(ItemData::default()); MAX_ITEMS],
            shops: vec![Arc::new(ShopData::default()); MAX_SHOPS],
        })
    }
}
