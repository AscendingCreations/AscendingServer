use mmap_bytey::MByteBuffer;

use crate::{
    npcs::Npc, players::Player, ClaimsKey, GameTime, GlobalKey, MapPosition, Position, UserAccess,
};

use super::{DropItem, MapItem};

#[derive(Clone, Debug)]
pub enum MapIncomming {
    SpawnNpc {
        npc: Box<Npc>,
        claimkey: ClaimsKey,
    },
    SpawnMapItem {
        item: Box<MapItem>,
        claimkey: ClaimsKey,
    },
    SpawnPlayer {
        player: Box<Player>,
    },
    SendBlockUpdate {
        map_id: MapPosition,
        x: u32,
        y: u32,
        blocked: bool,
    },
    VerifyPlayerMove {
        map_id: MapPosition,
        position: Position,
        id: GlobalKey,
    },
    MovePlayer {
        map_id: MapPosition,
        player: Box<Player>,
    },
    PlayerMessage {
        map_id: MapPosition,
    },
    DropItem {
        map_id: MapPosition,
        item: DropItem,
        claim_id: GlobalKey,
    },
    RequestItemDrop {
        map_id: MapPosition,
        item: DropItem,
        channel: tokio::sync::mpsc::Sender<MapQuickResponse>,
    },
    SendPacketToAll {
        map_id: MapPosition,
        buffer: MByteBuffer,
        avoid: Option<GlobalKey>,
    },
}

#[derive(Clone, Debug)]
pub enum MapBroadCasts {
    PlayerLoggedIn {
        map_id: MapPosition,
        key: GlobalKey,
        username: String,
        access: UserAccess,
        position: Position,
    },
    PlayerLoggedOut {
        map_id: MapPosition,
        key: GlobalKey,
        username: String,
        position: Position,
    },
    PlayerMessage {
        map_id: MapPosition,
        username: String,
        message: String,
        sender_name: String,
        sender_access: UserAccess,
        sender_id: GlobalKey,
    },
    TimeUpdate {
        time: GameTime,
    },
    SendPacketToAll {
        map_id: MapPosition,
        buffer: MByteBuffer,
    },
}

#[derive(Clone, Debug)]
pub enum MapQuickResponse {
    None,
    DropItem {
        map_id: MapPosition,
        item: DropItem,
        drop_amount: u16,
        claim_id: GlobalKey,
    },
}
