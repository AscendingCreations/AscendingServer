use super::routes;
use crate::{containers::Storage, gametypes::*};
use bytey::{ByteBuffer, ByteBufferRead, ByteBufferWrite};
use hecs::World;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type PacketFunction = fn(&mut World, &Storage, &mut ByteBuffer, &Entity) -> Result<()>;

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
    SwitchStorageSlot,
    DeleteStorageItem,
    DepositItem,
    WithdrawItem,
    Message,
    AdminCommand,
    SetTarget,
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
                ClientPacket::SwitchStorageSlot,
                routes::handle_switchstorageslot as PacketFunction,
            ),
            (
                ClientPacket::DeleteStorageItem,
                routes::handle_deletestorageitem as PacketFunction,
            ),
            (
                ClientPacket::DepositItem,
                routes::handle_deposititem as PacketFunction,
            ),
            (
                ClientPacket::WithdrawItem,
                routes::handle_withdrawitem as PacketFunction,
            ),
            (
                ClientPacket::Message,
                routes::handle_message as PacketFunction,
            ),
            (
                ClientPacket::AdminCommand,
                routes::handle_admincommand as PacketFunction,
            ),
            (
                ClientPacket::SetTarget,
                routes::handle_settarget as PacketFunction,
            ),
        ]))
    }
}
