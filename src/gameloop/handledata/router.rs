use super::packet_mapper;
use crate::{
    containers::Storage, gametypes::Result, socket::*, AscendingError, Entity, OnlineType,
    WorldExtras,
};
use hecs::World;

pub async fn handle_data(
    world: &mut World,
    storage: &Storage,
    data: &mut MByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let id: ClientPacket = data.read()?;

    let onlinetype = world.get_or_err::<OnlineType>(entity)?;

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
