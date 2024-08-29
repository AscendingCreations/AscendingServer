use super::packet_mapper;
use crate::{
    containers::{GameStore, GameWorld},
    gametypes::Result,
    socket::*,
    AscendingError, Entity, OnlineType, WorldExtrasAsync,
};

pub async fn handle_data(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let id: ClientPacket = data.read()?;

    let onlinetype = world.get_or_err::<OnlineType>(entity).await?;

    match onlinetype {
        OnlineType::Online => match id {
            ClientPacket::Login
            | ClientPacket::Register
            | ClientPacket::HandShake
            | ClientPacket::OnlineCheck => return Err(AscendingError::MultiLogin),
            _ => {}
        },
        OnlineType::Accepted => match id {
            ClientPacket::Login
            | ClientPacket::Register
            | ClientPacket::OnlineCheck
            | ClientPacket::HandShake
            | ClientPacket::Ping => {}
            _ => return Err(AscendingError::PacketManipulation { name: "".into() }),
        },
        OnlineType::None => return Err(AscendingError::PacketManipulation { name: "".into() }),
    }

    packet_mapper(world, storage, data, entity, id).await
}
