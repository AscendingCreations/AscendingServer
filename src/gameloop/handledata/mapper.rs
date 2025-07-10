use super::{
    SocketID, handle_account::*, handle_action::*, handle_general::*, handle_item::*,
    handle_trade::*,
};
use crate::{
    containers::{GlobalKey, Storage, World},
    gametypes::*,
    socket::*,
};

type PacketFunction =
    fn(&mut World, &Storage, &mut MByteBuffer, Option<GlobalKey>, SocketID) -> Result<()>;

pub fn run_packet(packet: &ClientPacket) -> Option<PacketFunction> {
    match packet {
        ClientPacket::Register => Some(handle_register as PacketFunction),
        ClientPacket::Login => Some(handle_login as PacketFunction),
        ClientPacket::Move => Some(handle_move as PacketFunction),
        ClientPacket::Dir => Some(handle_dir as PacketFunction),
        ClientPacket::Attack => Some(handle_attack as PacketFunction),
        ClientPacket::UseItem => Some(handle_useitem as PacketFunction),
        ClientPacket::Unequip => Some(handle_unequip as PacketFunction),
        ClientPacket::SwitchInvSlot => Some(handle_switchinvslot as PacketFunction),
        ClientPacket::PickUp => Some(handle_pickup as PacketFunction),
        ClientPacket::DropItem => Some(handle_dropitem as PacketFunction),
        ClientPacket::DeleteItem => Some(handle_deleteitem as PacketFunction),
        ClientPacket::SwitchStorageSlot => Some(handle_switchstorageslot as PacketFunction),
        ClientPacket::DeleteStorageItem => Some(handle_deletestorageitem as PacketFunction),
        ClientPacket::DepositItem => Some(handle_deposititem as PacketFunction),
        ClientPacket::WithdrawItem => Some(handle_withdrawitem as PacketFunction),
        ClientPacket::Message => Some(handle_message as PacketFunction),
        ClientPacket::Command => Some(handle_command as PacketFunction),
        ClientPacket::SetTarget => Some(handle_settarget as PacketFunction),
        ClientPacket::CloseStorage => Some(handle_closestorage as PacketFunction),
        ClientPacket::CloseShop => Some(handle_closeshop as PacketFunction),
        ClientPacket::CloseTrade => Some(handle_closetrade as PacketFunction),
        ClientPacket::BuyItem => Some(handle_buyitem as PacketFunction),
        ClientPacket::SellItem => Some(handle_sellitem as PacketFunction),
        ClientPacket::AddTradeItem => Some(handle_addtradeitem as PacketFunction),
        ClientPacket::RemoveTradeItem => Some(handle_removetradeitem as PacketFunction),
        ClientPacket::UpdateTradeMoney => Some(handle_updatetrademoney as PacketFunction),
        ClientPacket::SubmitTrade => Some(handle_submittrade as PacketFunction),
        ClientPacket::HandShake => Some(handle_handshake as PacketFunction),
        ClientPacket::AcceptTrade => Some(handle_accepttrade as PacketFunction),
        ClientPacket::DeclineTrade => Some(handle_declinetrade as PacketFunction),
        ClientPacket::Ping => Some(handle_ping as PacketFunction),
        ClientPacket::TlsReconnect => Some(handle_tls_reconnect as PacketFunction),
        ClientPacket::TlsHandShake => Some(handle_tls_handshake as PacketFunction),
        ClientPacket::Disconnect => Some(handle_disconnect as PacketFunction),
        ClientPacket::Reconnect => Some(handle_reconnect as PacketFunction),
        ClientPacket::LoginOk => Some(handle_login_ok as PacketFunction),
        ClientPacket::OnlineCheck => None,
    }
}
