use super::routes;
use crate::{containers::Storage, gametypes::*};
use bytey::{ByteBuffer, ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type PacketFunction = fn(&mut hecs::World, &Storage, &mut ByteBuffer, &Entity) -> Result<()>;

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ByteBufferRead, ByteBufferWrite, Hash,
)]
pub enum ClientPacket {
    Register,
    Login,
    Move,
    Dir,
    Attack,
    UseItem,
    Unequip,
    SwitchInvSlot,
    PickUp,
    DropItem,
    DeleteItem,
    Message,
    Size,
}

pub struct PacketRouter(pub HashMap<ClientPacket, PacketFunction>);

impl PacketRouter {
    pub fn init() -> Self {
        Self(HashMap::from([
            (
                ClientPacket::Register,
                routes::handle_register as PacketFunction,
            ),
            (ClientPacket::Login, routes::handle_login as PacketFunction),
            (ClientPacket::Move, routes::handle_move as PacketFunction),
            (ClientPacket::Dir, routes::handle_dir as PacketFunction),
            (
                ClientPacket::Attack,
                routes::handle_attack as PacketFunction,
            ),
            (
                ClientPacket::UseItem,
                routes::handle_useitem as PacketFunction,
            ),
            (
                ClientPacket::Unequip,
                routes::handle_unequip as PacketFunction,
            ),
            (
                ClientPacket::SwitchInvSlot,
                routes::handle_switchinvslot as PacketFunction,
            ),
            (
                ClientPacket::PickUp,
                routes::handle_pickup as PacketFunction,
            ),
            (
                ClientPacket::DropItem,
                routes::handle_dropitem as PacketFunction,
            ),
            (
                ClientPacket::DeleteItem,
                routes::handle_deleteitem as PacketFunction,
            ),
            (
                ClientPacket::Message,
                routes::handle_message as PacketFunction,
            ),
        ]))
    }
}
