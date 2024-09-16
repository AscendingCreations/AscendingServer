use crate::{players::Player, EntityKey, GameTime, MapPosition, Position, UserAccess};

use super::DropItem;

#[derive(Clone, Debug)]
pub enum MapIncomming {
    SendBlockUpdate {
        map_id: MapPosition,
        x: u32,
        y: u32,
        blocked: bool,
    },
    VerifyPlayerMove {
        map_id: MapPosition,
        position: Position,
        id: EntityKey,
    },
    MovePlayer {
        map_id: MapPosition,
        player: Box<Player>,
        old_id: EntityKey,
    },
    PlayerMessage {
        map_id: MapPosition,
    },
    DropItem {
        map_id: MapPosition,
        item: DropItem,
        claim_id: EntityKey,
    },
    RequestItemDrop {
        map_id: MapPosition,
        item: DropItem,
        channel: tokio::sync::mpsc::Sender<MapQuickResponse>,
    },
}

#[derive(Clone, Debug)]
pub enum MapBroadCasts {
    PlayerLoggedIn {
        map_id: MapPosition,
        username: String,
        access: UserAccess,
        position: Position,
    },
    PlayerLoggedOut {
        map_id: MapPosition,
        username: String,
        position: Position,
    },
    PlayerMessage {
        map_id: MapPosition,
        username: String,
        message: String,
        sender_name: String,
        sender_access: UserAccess,
        sender_id: EntityKey,
    },
    MovePlayer {
        map_id: MapPosition,
        player: Box<Player>,
        old_id: EntityKey,
    },
    TimeUpdate {
        time: GameTime,
    },
}

#[derive(Clone, Debug)]
pub enum MapQuickResponse {
    None,
    DropItem {
        map_id: MapPosition,
        item: DropItem,
        drop_amount: u16,
        claim_id: EntityKey,
    },
}
