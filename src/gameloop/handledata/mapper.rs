use super::routes;
use crate::{containers::Storage, gametypes::*, socket::*};
use hecs::World;
use std::collections::HashMap;

type PacketFunction = fn(&mut World, &Storage, &mut ByteBuffer, &Entity) -> Result<()>;

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
                ClientPacket::Command,
                routes::handle_command as PacketFunction,
            ),
            (
                ClientPacket::SetTarget,
                routes::handle_settarget as PacketFunction,
            ),
            (
                ClientPacket::CloseStorage,
                routes::handle_closestorage as PacketFunction,
            ),
            (
                ClientPacket::CloseShop,
                routes::handle_closeshop as PacketFunction,
            ),
            (
                ClientPacket::CloseTrade,
                routes::handle_closetrade as PacketFunction,
            ),
            (
                ClientPacket::BuyItem,
                routes::handle_buyitem as PacketFunction,
            ),
            (
                ClientPacket::SellItem,
                routes::handle_sellitem as PacketFunction,
            ),
            (
                ClientPacket::AddTradeItem,
                routes::handle_addtradeitem as PacketFunction,
            ),
            (
                ClientPacket::RemoveTradeItem,
                routes::handle_removetradeitem as PacketFunction,
            ),
            (
                ClientPacket::UpdateTradeMoney,
                routes::handle_updatetrademoney as PacketFunction,
            ),
            (
                ClientPacket::SubmitTrade,
                routes::handle_submittrade as PacketFunction,
            ),
            (
                ClientPacket::HandShake,
                routes::handle_handshake as PacketFunction,
            ),
        ]))
    }
}
