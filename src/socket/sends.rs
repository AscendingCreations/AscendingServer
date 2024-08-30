use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    players::*,
    socket::*,
    tasks::*,
};
use std::ops::Range;

#[inline]
pub async fn send_infomsg(
    storage: &GameStore,
    socket_id: usize,
    message: String,
    close_socket: u8,
    tls_send: bool,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::AlertMsg)?;
    buf.write(message)?;
    buf.write(close_socket)?;
    buf.finish()?;

    if tls_send {
        tls_send_to(storage, socket_id, buf).await
    } else {
        send_to(storage, socket_id, buf).await
    }
}

#[inline]
pub async fn send_fltalert(
    storage: &GameStore,
    socket_id: usize,
    message: String,
    ftltype: FtlType,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::FltAlert)?;
    buf.write(ftltype)?;
    buf.write(message)?;
    buf.finish()?;

    send_to(storage, socket_id, buf).await
}

#[inline]
pub async fn send_loginok(storage: &GameStore, socket_id: usize) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    let time = storage.time.lock().await;

    buf.write(ServerPackets::LoginOk)?;
    buf.write(time.hour)?;
    buf.write(time.min)?;
    buf.finish()?;

    send_to(storage, socket_id, buf).await
}

#[inline]
pub async fn send_myindex(storage: &GameStore, socket_id: usize, entity: &Entity) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::MyIndex)?;
    buf.write(*entity)?;
    buf.write(*entity)?;
    buf.finish()?;

    tls_send_to(storage, socket_id, buf).await
}

pub async fn send_move_ok(storage: &GameStore, socket_id: usize, move_ok: bool) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::MoveOk)?;
    buf.write(move_ok)?;
    buf.finish()?;

    send_to_front(storage, socket_id, buf).await
}

#[inline]
pub async fn send_playerdata(
    world: &GameWorld,
    storage: &GameStore,
    socket_id: usize,
    entity: &Entity,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    let username = {
        let lock = world.read().await;
        let username = lock.get::<&Account>(entity.0)?.username.clone();
        username
    };

    buf.write(ServerPackets::PlayerData)?;
    buf.write(username)?;
    buf.write(world.get_or_err::<UserAccess>(entity).await?)?;
    buf.write(world.get_or_err::<Dir>(entity).await?.0)?;
    let equipment = world.cloned_get_or_err::<Equipment>(entity).await?;
    buf.write(equipment)?;
    buf.write(world.get_or_err::<Hidden>(entity).await?.0)?;
    buf.write(world.get_or_err::<Level>(entity).await?.0)?;
    buf.write(world.get_or_err::<DeathType>(entity).await?)?;
    buf.write(world.get_or_err::<Physical>(entity).await?.damage)?;
    buf.write(world.get_or_err::<Physical>(entity).await?.defense)?;
    buf.write(world.get_or_err::<Position>(entity).await?)?;
    buf.write(world.get_or_err::<Player>(entity).await?.pk)?;
    buf.write(world.get_or_err::<Player>(entity).await?.pvpon)?;
    buf.write(world.get_or_err::<Sprite>(entity).await?.id as u8)?;
    buf.write(world.get_or_err::<Vitals>(entity).await?.vital)?;
    buf.write(world.get_or_err::<Vitals>(entity).await?.vitalmax)?;
    buf.finish()?;

    send_to(storage, socket_id, buf).await
}

pub async fn send_codes(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    code: String,
    handshake: String,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::HandShake)?;
    buf.write(&code)?;
    buf.write(&handshake)?;
    buf.finish()?;

    let lock = world.read().await;
    let id = lock.get::<&Socket>(entity.0)?.id;

    // Once the codes are Sent we need to set this to unencrypted mode as the client will be un unencrypted mode.
    set_encryption_status(storage, id, EncryptionState::WriteTransfering).await;
    tls_send_to(storage, id, buf).await
}

pub async fn send_ping(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OnlineCheck)?;
    buf.write(0u64)?;
    buf.finish()?;

    let lock = world.read().await;
    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_inv(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;
    let lock = world.read().await;

    buf.write(ServerPackets::PlayerInv)?;
    buf.write(&lock.get::<&Inventory>(entity.0)?.items)?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_invslot(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    id: usize,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;
    let lock = world.read().await;

    buf.write(ServerPackets::PlayerInvSlot)?;
    buf.write(id)?;
    buf.write(lock.get::<&Inventory>(entity.0)?.items[id])?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_storage(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    range: Range<usize>,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;
    let lock = world.read().await;
    let storage_slots = lock.get::<&PlayerStorage>(entity.0)?;

    buf.write(ServerPackets::PlayerStorage)?;
    buf.write(range.clone())?;
    buf.write(&storage_slots.items[range])?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_storageslot(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    id: usize,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;
    let lock = world.read().await;

    buf.write(ServerPackets::PlayerStorageSlot)?;
    buf.write(id)?;
    buf.write(lock.get::<&PlayerStorage>(entity.0)?.items[id])?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_equipment(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerEquipment)?;
    buf.write(*entity)?;
    buf.write(world.cloned_get_or_err::<Equipment>(entity).await?)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_err::<Position>(entity).await?.map,
        buf,
        None,
    )
    .await
}

#[inline]
pub async fn send_level(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerLevel)?;
    buf.write(world.get_or_err::<Level>(entity).await?.0)?;
    buf.write(world.get_or_err::<Player>(entity).await?.levelexp)?;
    buf.finish()?;

    let lock = world.read().await;
    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_money(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerMoney)?;
    buf.write(world.get_or_err::<Money>(entity).await?.vals)?;
    buf.finish()?;

    let lock = world.read().await;
    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_pk(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    toself: bool,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPk)?;
    buf.write(world.get_or_err::<Player>(entity).await?.pk)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_err::<Position>(entity).await?.map,
        buf,
        closure(toself, *entity),
    )
    .await
}

#[inline]
pub async fn send_message(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    msg: String,
    head: String,
    chan: MessageChannel,
    id: Option<usize>,
) -> Result<()> {
    let access = world.get_or_err::<UserAccess>(entity).await?;

    match chan {
        MessageChannel::Map => {
            DataTaskToken::MapChat(world.get_or_err::<Position>(entity).await?.map)
                .add_task(storage, message_packet(chan, head, msg, Some(access))?)
                .await?
        }
        MessageChannel::Global => {
            DataTaskToken::GlobalChat
                .add_task(storage, message_packet(chan, head, msg, Some(access))?)
                .await?
        }
        MessageChannel::Party | MessageChannel::Trade | MessageChannel::Help => {}
        MessageChannel::Private => {
            let mut buf = MByteBuffer::new_packet()?;
            buf.write(ServerPackets::ChatMsg)?;
            buf.write(1_u32)?;
            buf.write(chan)?;
            buf.write(head)?;
            buf.write(msg)?;
            buf.write(Some(access))?;
            buf.finish()?;

            if let Some(i) = id {
                send_to(storage, i, buf.try_clone()?).await?;
            }

            let lock = world.read().await;
            let id = lock.get::<&Socket>(entity.0)?.id;
            send_to(storage, id, buf).await?;
        }
        MessageChannel::Guild => {}
        MessageChannel::Quest | MessageChannel::Npc => {
            let mut buf = MByteBuffer::new_packet()?;

            buf.write(ServerPackets::ChatMsg)?;
            buf.write(1_u32)?;
            buf.write(chan)?;
            buf.write(head)?;
            buf.write(msg)?;
            buf.write(Some(access))?;
            buf.finish()?;

            let lock = world.read().await;
            let id = lock.get::<&Socket>(entity.0)?.id;
            send_to(storage, id, buf).await?;
        }
    }

    Ok(())
}

#[inline]
pub async fn send_openstorage(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OpenStorage)?;
    buf.write(1_u32)?;
    buf.finish()?;

    let lock = world.read().await;
    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_openshop(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    shop_index: u16,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OpenShop)?;
    buf.write(shop_index)?;
    buf.finish()?;

    let lock = world.read().await;
    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_clearisusingtype(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::ClearIsUsingType)?;
    buf.write(1_u16)?;
    buf.finish()?;

    let lock = world.read().await;
    let id = lock.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_updatetradeitem(
    world: &GameWorld,
    storage: &GameStore,
    target_entity: &Entity,
    send_entity: &Entity,
    trade_slot: u16,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    let lock = world.read().await;
    buf.write(ServerPackets::UpdateTradeItem)?;
    buf.write(target_entity == send_entity)?;
    buf.write(trade_slot)?;
    buf.write(lock.get::<&TradeItem>(target_entity.0)?.items[trade_slot as usize])?;
    buf.finish()?;

    let id = lock.get::<&Socket>(send_entity.0)?.id;
    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_updatetrademoney(
    world: &GameWorld,
    storage: &GameStore,
    target_entity: &Entity,
    send_entity: &Entity,
) -> Result<()> {
    let lock = world.read().await;
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::UpdateTradeMoney)?;
    buf.write(lock.get::<&TradeMoney>(target_entity.0)?.vals)?;
    buf.finish()?;

    let id = lock.get::<&Socket>(send_entity.0)?.id;
    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_inittrade(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<()> {
    let lock = world.read().await;
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::InitTrade)?;
    buf.write(*target_entity)?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;
    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_tradestatus(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    my_status: &TradeStatus,
    their_status: &TradeStatus,
) -> Result<()> {
    let lock = world.read().await;
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::TradeStatus)?;
    buf.write(*my_status)?;
    buf.write(*their_status)?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;
    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_traderequest(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<()> {
    let lock = world.read().await;
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::TradeRequest)?;
    buf.write(*entity)?;
    buf.finish()?;

    let id = lock.get::<&Socket>(target_entity.0)?.id;
    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_playitemsfx(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    item_index: u16,
) -> Result<()> {
    let lock = world.read().await;
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayItemSfx)?;
    buf.write(item_index)?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;
    send_to(storage, id, buf).await
}

#[inline]
pub async fn send_gameping(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let lock = world.read().await;
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::Ping)?;
    buf.write(0u64)?;
    buf.finish()?;

    let id = lock.get::<&Socket>(entity.0)?.id;
    send_to(storage, id, buf).await
}
