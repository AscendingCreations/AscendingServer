use mio::Token;

use crate::{
    AscendingError, PacketRouter,
    containers::{GlobalKey, Storage, World},
    gametypes::Result,
    socket::*,
};

pub struct SocketID {
    pub id: Token,
    pub is_tls: bool,
}

pub fn handle_data(
    router: &PacketRouter,
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let id: ClientPacket = data.read()?;

    if entity.is_some() {
        match id {
            ClientPacket::Login
            | ClientPacket::Register
            | ClientPacket::HandShake
            | ClientPacket::OnlineCheck => return Err(AscendingError::MultiLogin),
            _ => {}
        }
    } else {
        match id {
            ClientPacket::Login
            | ClientPacket::Register
            | ClientPacket::OnlineCheck
            | ClientPacket::HandShake
            | ClientPacket::Ping => {}
            _ => return Err(AscendingError::PacketManipulation { name: "".into() }),
        }
    }

    if id == ClientPacket::OnlineCheck {
        return Ok(());
    }

    let fun = match router.0.get(&id) {
        Some(fun) => fun,
        None => return Err(AscendingError::InvalidPacket),
    };

    fun(world, storage, data, entity, socket_id)
}
