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
            npcs: {
                let mut v = Vec::with_capacity(MAX_NPCS);
                (0..MAX_NPCS).for_each(|_| v.push(Arc::new(NpcData::default())));
                v
            },
            items: {
                let mut v = Vec::with_capacity(MAX_ITEMS);
                (0..MAX_ITEMS).for_each(|_| v.push(Arc::new(ItemData::default())));
                v
            },
            shops: {
                let mut v = Vec::with_capacity(MAX_SHOPS);
                (0..MAX_SHOPS).for_each(|_| v.push(Arc::new(ShopData::default())));
                v
            },
        })
    }
}
