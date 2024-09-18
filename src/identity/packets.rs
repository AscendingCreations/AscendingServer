use super::{ClaimsKey, GlobalKey};
use crate::{maps::MapItem, npcs::Npc, players::Player, MapPosition};

#[derive(Debug)]
pub enum IDIncomming {
    RequestNpcSpawn {
        spawn_map: MapPosition,
        npc: Box<Npc>,
        claim: ClaimsKey,
    },
    RequestPlayerSpawn {
        spawn_map: MapPosition,
        player: Box<Player>,
    },
    RequestItemSpawn {
        spawn_map: MapPosition,
        item: Box<MapItem>,
        claim: ClaimsKey,
    },
    RemoveEntity {
        key: GlobalKey,
    },
}
