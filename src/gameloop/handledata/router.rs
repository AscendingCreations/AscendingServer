use crate::{
    containers::Storage, gametypes::Result, AscendingError, ClientPacket, Entity, OnlineType,
    PacketRouter, WorldExtras,
};
use bytey::ByteBuffer;

pub fn handle_data(
    router: &PacketRouter,
    world: &mut hecs::World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let id: ClientPacket = data.read()?;

    let onlinetype = world.get_or_panic::<OnlineType>(entity);

    match onlinetype {
        OnlineType::Online => {
            if id == ClientPacket::Login {
                return Err(AscendingError::MultiLogin);
            }
        }
        OnlineType::Accepted => match id {
            ClientPacket::Login | ClientPacket::Register => {}
            _ => return Err(AscendingError::PacketManipulation { name: "".into() }),
        },
        OnlineType::None => {
            return Err(AscendingError::PacketManipulation { name: "".into() });
        }
    }

    let fun = match router.0.get(&id) {
        Some(fun) => fun,
        None => return Err(AscendingError::InvalidPacket),
    };

    fun(world, storage, data, entity)
}
