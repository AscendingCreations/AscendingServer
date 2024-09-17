use super::{ClaimsKey, GlobalKey};
use crate::{maps::MapItem, npcs::Npc, players::Player, MapPosition};

#[derive(Debug)]
pub enum IDIncomming {
    RequestNpcSpawn {
        spawn_map: MapPosition,
        npc: Npc,
        claim: ClaimsKey,
    },
    RequestPlayerSpawn {
        spawn_map: MapPosition,
        player: Player,
    },
    RequestItemSpawn {
        spawn_map: MapPosition,
        item: MapItem,
        claim: ClaimsKey,
    },
    RemoveEntity {
        key: GlobalKey,
    },
}
