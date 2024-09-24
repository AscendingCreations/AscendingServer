use super::routes;
use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    network::*,
};

pub async fn packet_mapper(
    world: &GameWorld,
    storage: &GameStore,
    data: &mut MByteBuffer,
    entity: &GlobalKey,
    id: ClientPacketID,
) -> Result<()> {
    match id {
        ClientPacketID::Login => routes::handle_login(world, storage, data, entity).await,
        ClientPacketID::Move => routes::handle_move(world, storage, data, entity).await,
        ClientPacketID::Dir => routes::handle_dir(world, storage, data, entity).await,
        ClientPacketID::Attack => routes::handle_attack(world, storage, data, entity).await,
        ClientPacketID::UseItem => routes::handle_useitem(world, storage, data, entity).await,
        ClientPacketID::Unequip => routes::handle_unequip(world, storage, data, entity).await,
        ClientPacketID::SwitchInvSlot => {
            routes::handle_switchinvslot(world, storage, data, entity).await
        }
        ClientPacketID::PickUp => routes::handle_pickup(world, storage, data, entity).await,
        ClientPacketID::DropItem => routes::handle_dropitem(world, storage, data, entity).await,
        ClientPacketID::DeleteItem => routes::handle_deleteitem(world, storage, data, entity).await,
        ClientPacketID::SwitchStorageSlot => {
            routes::handle_switchstorageslot(world, storage, data, entity).await
        }
        ClientPacketID::DeleteStorageItem => {
            routes::handle_deletestorageitem(world, storage, data, entity).await
        }
        ClientPacketID::DepositItem => {
            routes::handle_deposititem(world, storage, data, entity).await
        }
        ClientPacketID::WithdrawItem => {
            routes::handle_withdrawitem(world, storage, data, entity).await
        }
        ClientPacketID::Message => routes::handle_message(world, storage, data, entity).await,
        ClientPacketID::Command => routes::handle_command(world, storage, data, entity).await,
        ClientPacketID::SetTarget => routes::handle_settarget(world, storage, data, entity).await,
        ClientPacketID::CloseStorage => {
            routes::handle_closestorage(world, storage, data, entity).await
        }
        ClientPacketID::CloseShop => routes::handle_closeshop(world, storage, data, entity).await,
        ClientPacketID::CloseTrade => routes::handle_closetrade(world, storage, data, entity).await,
        ClientPacketID::BuyItem => routes::handle_buy_item(world, storage, data, entity).await,
        ClientPacketID::SellItem => routes::handle_sellitem(world, storage, data, entity).await,
        ClientPacketID::AddTradeItem => {
            routes::handle_addtradeitem(world, storage, data, entity).await
        }
        ClientPacketID::RemoveTradeItem => {
            routes::handle_removetradeitem(world, storage, data, entity).await
        }
        ClientPacketID::UpdateTradeMoney => {
            routes::handle_updatetrademoney(world, storage, data, entity).await
        }
        ClientPacketID::SubmitTrade => {
            routes::handle_submittrade(world, storage, data, entity).await
        }
        ClientPacketID::HandShake => routes::handle_handshake(world, storage, data, entity).await,
        ClientPacketID::AcceptTrade => {
            routes::handle_accepttrade(world, storage, data, entity).await
        }
        ClientPacketID::DeclineTrade => {
            routes::handle_declinetrade(world, storage, data, entity).await
        }
        ClientPacketID::Ping => routes::handle_ping(world, storage, data, entity).await,
        ClientPacketID::OnlineCheck => Ok(()),
    }
}
