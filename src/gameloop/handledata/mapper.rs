use super::routes;
use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    socket::*,
};

pub async fn packet_mapper(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &Entity,
    id: ClientPacket,
) -> Result<()> {
    match id {
        ClientPacket::Register => routes::handle_register(world, storage, data, entity).await,
        ClientPacket::Login => routes::handle_login(world, storage, data, entity).await,
        ClientPacket::Move => routes::handle_move(world, storage, data, entity).await,
        ClientPacket::Dir => routes::handle_dir(world, storage, data, entity).await,
        ClientPacket::Attack => routes::handle_attack(world, storage, data, entity).await,
        ClientPacket::UseItem => routes::handle_useitem(world, storage, data, entity).await,
        ClientPacket::Unequip => routes::handle_unequip(world, storage, data, entity).await,
        ClientPacket::SwitchInvSlot => {
            routes::handle_switchinvslot(world, storage, data, entity).await
        }
        ClientPacket::PickUp => routes::handle_pickup(world, storage, data, entity).await,
        ClientPacket::DropItem => routes::handle_dropitem(world, storage, data, entity).await,
        ClientPacket::DeleteItem => routes::handle_deleteitem(world, storage, data, entity).await,
        ClientPacket::SwitchStorageSlot => {
            routes::handle_switchstorageslot(world, storage, data, entity).await
        }
        ClientPacket::DeleteStorageItem => {
            routes::handle_deletestorageitem(world, storage, data, entity).await
        }
        ClientPacket::DepositItem => routes::handle_deposititem(world, storage, data, entity).await,
        ClientPacket::WithdrawItem => {
            routes::handle_withdrawitem(world, storage, data, entity).await
        }
        ClientPacket::Message => routes::handle_message(world, storage, data, entity).await,
        ClientPacket::Command => routes::handle_command(world, storage, data, entity).await,
        ClientPacket::SetTarget => routes::handle_settarget(world, storage, data, entity).await,
        ClientPacket::CloseStorage => {
            routes::handle_closestorage(world, storage, data, entity).await
        }
        ClientPacket::CloseShop => routes::handle_closeshop(world, storage, data, entity).await,
        ClientPacket::CloseTrade => routes::handle_closetrade(world, storage, data, entity).await,
        ClientPacket::BuyItem => routes::handle_buyitem(world, storage, data, entity).await,
        ClientPacket::SellItem => routes::handle_sellitem(world, storage, data, entity).await,
        ClientPacket::AddTradeItem => {
            routes::handle_addtradeitem(world, storage, data, entity).await
        }
        ClientPacket::RemoveTradeItem => {
            routes::handle_removetradeitem(world, storage, data, entity).await
        }
        ClientPacket::UpdateTradeMoney => {
            routes::handle_updatetrademoney(world, storage, data, entity).await
        }
        ClientPacket::SubmitTrade => routes::handle_submittrade(world, storage, data, entity).await,
        ClientPacket::HandShake => routes::handle_handshake(world, storage, data, entity).await,
        ClientPacket::AcceptTrade => routes::handle_accepttrade(world, storage, data, entity).await,
        ClientPacket::DeclineTrade => {
            routes::handle_declinetrade(world, storage, data, entity).await
        }
        ClientPacket::Ping => routes::handle_ping(world, storage, data, entity).await,
        ClientPacket::OnlineCheck => Ok(()),
    }
}
