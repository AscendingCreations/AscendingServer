use crate::{containers::IndexMap, gametypes::*, items::*, maps::*, npcs::*};
pub struct Bases {
    pub map: IndexMap<MapPosition, Map>,
    pub npc: Vec<NpcData>,
    pub item: Vec<ItemData>,
}

impl Bases {
    pub fn new() -> Option<Self> {
        Some(Self {
            map: IndexMap::default(),
            npc: vec![NpcData::default(); MAX_NPCS],
            item: vec![ItemData::default(); MAX_ITEMS],
        })
    }
}
