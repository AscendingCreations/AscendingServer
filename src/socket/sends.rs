use std::ops::Range;

use crate::{
    containers::{GlobalKey, Storage, World},
    gametypes::*,
    players::*,
    socket::*,
    tasks::*,
};

#[inline]
pub fn send_infomsg(
    storage: &Storage,
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
        tls_send_to(storage, socket_id, buf)
    } else {
        send_to(storage, socket_id, buf)
    }
}

#[inline]
pub fn send_fltalert(
    storage: &Storage,
    socket_id: usize,
    message: String,
    ftltype: FtlType,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::FltAlert)?;
    buf.write(ftltype)?;
    buf.write(message)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_loginok(storage: &Storage, socket_id: usize) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::LoginOk)?;
    buf.write(storage.time.borrow().hour)?;
    buf.write(storage.time.borrow().min)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_myindex(storage: &Storage, socket_id: usize, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::MyIndex)?;
    buf.write(*entity)?;
    buf.write(*entity)?;
    buf.finish()?;

    tls_send_to(storage, socket_id, buf)
}

pub fn send_move_ok(storage: &Storage, socket_id: usize, move_ok: bool) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::MoveOk)?;
    buf.write(move_ok)?;
    buf.finish()?;

    send_to_front(storage, socket_id, buf)
}

#[inline]
pub fn send_playerdata(
    world: &mut World,
    storage: &Storage,
    socket_id: usize,
    entity: GlobalKey,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerData)?;
    buf.write(world.get::<&Account>(entity.0)?.username.clone())?;
    buf.write(world.get_or_err::<UserAccess>(entity)?)?;
    buf.write(world.get_or_err::<Dir>(entity)?.0)?;
    let equipment = world.cloned_get_or_err::<Equipment>(entity)?;
    buf.write(equipment)?;
    buf.write(world.get_or_err::<Hidden>(entity)?.0)?;
    buf.write(world.get_or_err::<Level>(entity)?.0)?;
    buf.write(world.get_or_err::<DeathType>(entity)?)?;
    buf.write(world.get_or_err::<Physical>(entity)?.damage)?;
    buf.write(world.get_or_err::<Physical>(entity)?.defense)?;
    buf.write(world.get_or_err::<Position>(entity)?)?;
    buf.write(world.get_or_err::<Player>(entity)?.pk)?;
    buf.write(world.get_or_err::<Player>(entity)?.pvpon)?;
    buf.write(world.get_or_err::<Sprite>(entity)?.id as u8)?;
    buf.write(world.get_or_err::<Vitals>(entity)?.vital)?;
    buf.write(world.get_or_err::<Vitals>(entity)?.vitalmax)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

pub fn send_codes(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    code: String,
    handshake: String,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::HandShake)?;
    buf.write(&code)?;
    buf.write(&handshake)?;
    buf.finish()?;

    let id: usize = world.get::<&Socket>(entity.0)?.id;

    // Once the codes are Sent we need to set this to unencrypted mode as the client will be un unencrypted mode.
    set_encryption_status(storage, id, EncryptionState::WriteTransfering);
    tls_send_to(storage, id, buf)
}

pub fn send_ping(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OnlineCheck)?;
    buf.write(0u64)?;
    buf.finish()?;

    let id: usize = world.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf)
}

#[inline]
pub fn send_inv(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerInv)?;
    buf.write(&world.get::<&Inventory>(entity.0)?.items)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_invslot(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    id: usize,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerInvSlot)?;
    buf.write(id)?;
    buf.write(world.get::<&Inventory>(entity.0)?.items[id])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_storage(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    range: Range<usize>,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;
    let storage_slots = world.get::<&PlayerStorage>(entity.0)?;

    buf.write(ServerPackets::PlayerStorage)?;
    buf.write(range.clone())?;
    buf.write(&storage_slots.items[range])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_storageslot(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    id: usize,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerStorageSlot)?;
    buf.write(id)?;
    buf.write(world.get::<&PlayerStorage>(entity.0)?.items[id])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_equipment(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerEquipment)?;
    buf.write(*entity)?;
    buf.write(world.cloned_get_or_err::<Equipment>(entity)?)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_err::<Position>(entity)?.map,
        buf,
        None,
    )
}

#[inline]
pub fn send_level(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerLevel)?;
    buf.write(world.get_or_err::<Level>(entity)?.0)?;
    buf.write(world.get_or_err::<Player>(entity)?.levelexp)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_money(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayerMoney)?;
    buf.write(world.get_or_err::<Money>(entity)?.vals)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_pk(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    toself: bool,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPk)?;
    buf.write(world.get_or_err::<Player>(entity)?.pk)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_err::<Position>(entity)?.map,
        buf,
        closure(toself, *entity),
    )
}

#[inline]
pub fn send_message(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    msg: String,
    head: String,
    chan: MessageChannel,
    id: Option<usize>,
) -> Result<()> {
    let access = world.get_or_err::<UserAccess>(entity)?;

    match chan {
        MessageChannel::Map => DataTaskToken::MapChat(world.get_or_err::<Position>(entity)?.map)
            .add_task(storage, message_packet(chan, head, msg, Some(access))?)?,
        MessageChannel::Global => DataTaskToken::GlobalChat
            .add_task(storage, message_packet(chan, head, msg, Some(access))?)?,
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
                send_to(storage, i, buf.try_clone()?)?;
            }
            send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)?;
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
            send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)?;
        }
    }

    Ok(())
}

#[inline]
pub fn send_openstorage(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OpenStorage)?;
    buf.write(1_u32)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_openshop(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    shop_index: u16,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OpenShop)?;
    buf.write(shop_index)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_clearisusingtype(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::ClearIsUsingType)?;
    buf.write(1_u16)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_updatetradeitem(
    world: &mut World,
    storage: &Storage,
    target_entity: GlobalKey,
    send_entity: GlobalKey,
    trade_slot: u16,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::UpdateTradeItem)?;
    buf.write(target_entity == send_entity)?;
    buf.write(trade_slot)?;
    buf.write(world.get::<&TradeItem>(target_entity.0)?.items[trade_slot as usize])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(send_entity.0)?.id, buf)
}

#[inline]
pub fn send_updatetrademoney(
    world: &mut World,
    storage: &Storage,
    target_entity: GlobalKey,
    send_entity: GlobalKey,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::UpdateTradeMoney)?;
    buf.write(world.get::<&TradeMoney>(target_entity.0)?.vals)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(send_entity.0)?.id, buf)
}

#[inline]
pub fn send_inittrade(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::InitTrade)?;
    buf.write(*target_entity)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_tradestatus(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    my_status: &TradeStatus,
    their_status: &TradeStatus,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::TradeStatus)?;
    buf.write(*my_status)?;
    buf.write(*their_status)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_traderequest(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::TradeRequest)?;
    buf.write(*entity)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(target_entity.0)?.id, buf)
}

#[inline]
pub fn send_playitemsfx(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item_index: u16,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayItemSfx)?;
    buf.write(item_index)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_gameping(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::Ping)?;
    buf.write(0u64)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}
