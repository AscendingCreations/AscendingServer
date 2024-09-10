use super::packets::*;
use crate::{gametypes::Result, network::*};

pub fn packet_translator(data: &mut MByteBuffer) -> Result<ClientPacket> {
    let id: ClientPacketID = data.read()?;

    match id {
        ClientPacketID::Login => handle_login(data),
        ClientPacketID::Move => handle_move(data),
        ClientPacketID::Dir => handle_dir(data),
        ClientPacketID::Attack => handle_attack(data),
        ClientPacketID::UseItem => handle_useitem(data),
        ClientPacketID::Unequip => handle_unequip(data),
        ClientPacketID::SwitchInvSlot => handle_switchinvslot(data),
        ClientPacketID::PickUp => Ok(ClientPacket::PickUp),
        ClientPacketID::DropItem => handle_dropitem(data),
        ClientPacketID::DeleteItem => handle_deleteitem(data),
        ClientPacketID::SwitchStorageSlot => handle_switchstorageslot(data),
        ClientPacketID::DeleteStorageItem => handle_deletestorageitem(data),
        ClientPacketID::DepositItem => handle_deposititem(data),
        ClientPacketID::WithdrawItem => handle_withdrawitem(data),
        ClientPacketID::Message => handle_message(data),
        ClientPacketID::Command => handle_command(data),
        ClientPacketID::SetTarget => handle_settarget(data),
        ClientPacketID::CloseStorage => Ok(ClientPacket::CloseStorage),
        ClientPacketID::CloseShop => Ok(ClientPacket::CloseShop),
        ClientPacketID::CloseTrade => Ok(ClientPacket::CloseTrade),
        ClientPacketID::BuyItem => handle_buy_item(data),
        ClientPacketID::SellItem => handle_sellitem(data),
        ClientPacketID::AddTradeItem => handle_addtradeitem(data),
        ClientPacketID::RemoveTradeItem => handle_removetradeitem(data),
        ClientPacketID::UpdateTradeMoney => handle_updatetrademoney(data),
        ClientPacketID::SubmitTrade => Ok(ClientPacket::SubmitTrade),
        ClientPacketID::AcceptTrade => Ok(ClientPacket::AcceptTrade),
        ClientPacketID::DeclineTrade => Ok(ClientPacket::DeclineTrade),
        ClientPacketID::Ping => Ok(ClientPacket::Ping),
        ClientPacketID::OnlineCheck => Ok(ClientPacket::OnlineCheck),
    }
}
