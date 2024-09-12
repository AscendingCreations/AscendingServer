use crate::{players::Player, EntityKey, GameTime, MapPosition, Position, UserAccess};

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
        position: Player,
        old_id: EntityKey,
    },
    PlayerMessage {
        map_id: MapPosition,
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
        position: Player,
        old_id: EntityKey,
    },
    TimeUpdate {
        time: GameTime,
    },
}
