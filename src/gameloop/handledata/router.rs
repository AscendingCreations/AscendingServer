use mio::Token;

use crate::{
    AscendingError,
    containers::{GlobalKey, Storage, World},
    gameloop::handledata::mapper::run_packet,
    gametypes::Result,
    socket::*,
};

pub struct SocketID {
    pub id: Token,
    pub is_tls: bool,
}

pub fn handle_data(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: Option<GlobalKey>,
    socket_id: SocketID,
) -> Result<()> {
    let id: ClientPacket = data.read()?;

    if entity.is_some() {
        match id {
            ClientPacket::Login | ClientPacket::Register | ClientPacket::HandShake => {
                return Err(AscendingError::MultiLogin);
            }
            _ => {}
        }
    } else {
        match id {
            ClientPacket::Login
            | ClientPacket::Register
            | ClientPacket::OnlineCheck
            | ClientPacket::HandShake
            | ClientPacket::Ping
            | ClientPacket::TlsHandShake
            | ClientPacket::TlsReconnect => {}
            _ => return Err(AscendingError::PacketManipulation { name: "".into() }),
        }
    }

    if id == ClientPacket::OnlineCheck {
        return Ok(());
    }

    let fun = match run_packet(&id) {
        Some(fun) => fun,
        None => {
            println!("Packet {id:?}");
            return Err(AscendingError::InvalidPacket);
        }
    };

    if fun(world, storage, data, entity, socket_id).is_err() {
        println!("Packet {id:?}");
    }
    Ok(())
}
