use super::packet_mapper;
use crate::{
    containers::{GameStore, GameWorld},
    gametypes::Result,
    network::*,
    AscendingError, GlobalKey, OnlineType, WorldExtrasAsync,
};

pub async fn handle_data(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &GlobalKey,
) -> Result<()> {
    let id: ClientPacketID = data.read()?;

    let onlinetype = world.get_or_err::<OnlineType>(entity).await?;

    match onlinetype {
        OnlineType::Online => match id {
            ClientPacketID::Login
            | ClientPacketID::Register
            | ClientPacketID::HandShake
            | ClientPacketID::OnlineCheck => return Err(AscendingError::MultiLogin),
            _ => {}
        },
        OnlineType::Accepted => match id {
            ClientPacketID::Login
            | ClientPacketID::Register
            | ClientPacketID::OnlineCheck
            | ClientPacketID::HandShake
            | ClientPacketID::Ping => {}
            _ => return Err(AscendingError::PacketManipulation { name: "".into() }),
        },
        OnlineType::None => return Err(AscendingError::PacketManipulation { name: "".into() }),
    }

    packet_mapper(world, storage, data, entity, id).await
}
