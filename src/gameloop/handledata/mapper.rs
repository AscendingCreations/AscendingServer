use super::{
    SocketID, handle_account::*, handle_action::*, handle_general::*, handle_item::*,
    handle_trade::*,
};
use crate::{
    containers::{GlobalKey, Storage, World},
    gametypes::*,
    socket::*,
};
use std::collections::HashMap;

type PacketFunction =
    fn(&mut World, &Storage, &mut MByteBuffer, Option<GlobalKey>, SocketID) -> Result<()>;

pub struct PacketRouter(pub HashMap<ClientPacket, PacketFunction>);

impl PacketRouter {
    pub fn init() -> Self {
        Self(HashMap::from([
            (ClientPacket::Register, handle_register as PacketFunction),
            (ClientPacket::Login, handle_login as PacketFunction),
            (ClientPacket::Move, handle_move as PacketFunction),
            (ClientPacket::Dir, handle_dir as PacketFunction),
            (ClientPacket::Attack, handle_attack as PacketFunction),
            (ClientPacket::UseItem, handle_useitem as PacketFunction),
            (ClientPacket::Unequip, handle_unequip as PacketFunction),
            (
                ClientPacket::SwitchInvSlot,
                handle_switchinvslot as PacketFunction,
            ),
            (ClientPacket::PickUp, handle_pickup as PacketFunction),
            (ClientPacket::DropItem, handle_dropitem as PacketFunction),
            (
                ClientPacket::DeleteItem,
                handle_deleteitem as PacketFunction,
            ),
            (
                ClientPacket::SwitchStorageSlot,
                handle_switchstorageslot as PacketFunction,
            ),
            (
                ClientPacket::DeleteStorageItem,
                handle_deletestorageitem as PacketFunction,
            ),
            (
                ClientPacket::DepositItem,
                handle_deposititem as PacketFunction,
            ),
            (
                ClientPacket::WithdrawItem,
                handle_withdrawitem as PacketFunction,
            ),
            (ClientPacket::Message, handle_message as PacketFunction),
            (ClientPacket::Command, handle_command as PacketFunction),
            (ClientPacket::SetTarget, handle_settarget as PacketFunction),
            (
                ClientPacket::CloseStorage,
                handle_closestorage as PacketFunction,
            ),
            (ClientPacket::CloseShop, handle_closeshop as PacketFunction),
            (
                ClientPacket::CloseTrade,
                handle_closetrade as PacketFunction,
            ),
            (ClientPacket::BuyItem, handle_buyitem as PacketFunction),
            (ClientPacket::SellItem, handle_sellitem as PacketFunction),
            (
                ClientPacket::AddTradeItem,
                handle_addtradeitem as PacketFunction,
            ),
            (
                ClientPacket::RemoveTradeItem,
                handle_removetradeitem as PacketFunction,
            ),
            (
                ClientPacket::UpdateTradeMoney,
                handle_updatetrademoney as PacketFunction,
            ),
            (
                ClientPacket::SubmitTrade,
                handle_submittrade as PacketFunction,
            ),
            (ClientPacket::HandShake, handle_handshake as PacketFunction),
            (
                ClientPacket::AcceptTrade,
                handle_accepttrade as PacketFunction,
            ),
            (
                ClientPacket::DeclineTrade,
                handle_declinetrade as PacketFunction,
            ),
            (ClientPacket::Ping, handle_ping as PacketFunction),
        ]))
    }
}
