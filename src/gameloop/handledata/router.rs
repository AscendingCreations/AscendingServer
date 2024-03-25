use crate::{
    containers::Storage, gametypes::Result, AscendingError, ClientPacket, Entity, OnlineType,
    PacketRouter, WorldExtras,
};
use bytey::ByteBuffer;
use hecs::World;

pub fn handle_data(
    router: &PacketRouter,
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let id: ClientPacket = data.read()?;

    let onlinetype = world.get_or_panic::<OnlineType>(entity);

    match onlinetype {
        OnlineType::Online => match id {
            ClientPacket::Login | ClientPacket::Register => return Err(AscendingError::MultiLogin),
            _ => {}
        },
        OnlineType::Accepted => match id {
            ClientPacket::Login | ClientPacket::Register => {}
            _ => return Err(AscendingError::PacketManipulation { name: "".into() }),
        },
        OnlineType::None => return Err(AscendingError::PacketManipulation { name: "".into() }),
    }

    let fun = match router.0.get(&id) {
        Some(fun) => fun,
        None => return Err(AscendingError::InvalidPacket),
    };

    match fun(world, storage, data, entity) {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("Error {}", e);
            Err(e)
        }
    }
}
