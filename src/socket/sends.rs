use std::backtrace::Backtrace;

use crate::{containers::Storage, gametypes::*, maps::*, players::*, socket::*, tasks::*};
use hecs::World;

#[inline]
pub fn send_infomsg(
    storage: &Storage,
    socket_id: usize,
    message: String,
    close_socket: u8,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(128)?;

    buf.write(ServerPackets::AlertMsg)?;
    buf.write(message)?;
    buf.write(close_socket)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)?;
    Ok(())
}

#[inline]
pub fn send_fltalert(
    storage: &Storage,
    socket_id: usize,
    message: String,
    ftltype: FtlType,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(128)?;

    buf.write(ServerPackets::FltAlert)?;
    buf.write(ftltype)?;
    buf.write(message)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_loginok(storage: &Storage, socket_id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(12)?;

    buf.write(ServerPackets::LoginOk)?;
    buf.write(storage.time.borrow().hour)?;
    buf.write(storage.time.borrow().min)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_myindex(storage: &Storage, socket_id: usize, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(8)?;

    buf.write(ServerPackets::MyIndex)?;
    buf.write(*entity)?;
    buf.write(*entity)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_playerdata(
    world: &mut World,
    storage: &Storage,
    socket_id: usize,
    entity: &Entity,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(418)?;

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

#[inline]
pub fn send_updatemap(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::UpdateMap)?;
    buf.finish()?;

    let id: usize = world.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf)
}

#[inline]
pub fn send_mapitem(
    world: &mut World,
    storage: &Storage,
    position: MapPosition,
    id: Entity,
    sendto: Option<usize>,
) -> Result<()> {
    let map = match storage.maps.get(&position) {
        Some(map) => map,
        None => return Err(AscendingError::Unhandled(Box::new(Backtrace::capture()))),
    }
    .borrow();
    if let Some(item) = map.itemids.get(&id) {
        let itemdata = world.get_or_err::<MapItem>(item)?;

        let mut buf = ByteBuffer::new_packet_with(64)?;

        buf.write(ServerPackets::MapItems)?;
        buf.write(*item)?;
        buf.write(itemdata)?;
        buf.finish()?;

        return if let Some(socket_id) = sendto {
            send_to(storage, socket_id, buf)
        } else {
            send_to_maps(world, storage, position, buf, None)
        };
    }

    Ok(())
}

#[inline]
pub fn send_dir(world: &mut World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDir)?;
    buf.write(world.get_or_err::<Dir>(entity)?.0)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_err::<Position>(entity)?.map,
        buf,
        closure(toself, *entity),
    )
}

pub fn send_codes(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    code: String,
    handshake: String,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(code.len() + handshake.len() + 4)?;

    buf.write(ServerPackets::HandShake)?;
    buf.write(&code)?;
    buf.write(&handshake)?;
    buf.finish()?;

    let id: usize = world.get::<&Socket>(entity.0)?.id;

    // Once the codes are Sent we need to set this to unencrypted mode as the client will be un unencrypted mode.
    set_encryption_status(storage, id, EncryptionState::WriteTransfering);
    send_to(storage, id, buf)
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub fn send_move(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    pos: Position,
    warp: bool,
    switch: bool,
    dir: u8,
    send_to_pos: Option<Position>,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(31)?;

    buf.write(ServerPackets::PlayerMove)?;
    buf.write(*entity)?;
    buf.write(pos)?;
    buf.write(warp)?;
    buf.write(switch)?;
    buf.write(dir)?;
    buf.finish()?;

    if let Some(sendpos) = send_to_pos {
        send_to_maps(world, storage, sendpos.map, buf, Some(*entity))
    } else {
        send_to_maps(world, storage, pos.map, buf, Some(*entity))
    }
}

pub fn send_warp(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    let pos = world.get_or_err::<Position>(entity)?;

    buf.write(ServerPackets::PlayerWarp)?;
    buf.write(*entity)?;
    buf.write(pos)?;
    buf.finish()?;

    send_to_maps(world, storage, pos.map, buf, None)
}

pub fn send_data_remove_list(storage: &Storage, socket_id: usize, remove: &[Entity]) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(PACKET_DATA_LIMIT - 8)?;

    buf.write(ServerPackets::Dataremovelist)?;
    buf.write(remove.to_vec())?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_inv(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write(ServerPackets::PlayerInv)?;
    buf.write(&world.get::<&Inventory>(entity.0)?.items)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_invslot(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    id: usize,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write(ServerPackets::PlayerInvSlot)?;
    buf.write(id)?;
    buf.write(world.get::<&Inventory>(entity.0)?.items[id])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_storage(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write(ServerPackets::PlayerStorage)?;
    buf.write(&world.get::<&PlayerStorage>(entity.0)?.items)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_storageslot(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    id: usize,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write(ServerPackets::PlayerStorageSlot)?;
    buf.write(id)?;
    buf.write(world.get::<&PlayerStorage>(entity.0)?.items[id])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_attack(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    toself: bool,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAttack)?;
    buf.write(*entity)?;
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
pub fn send_equipment(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

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
pub fn send_level(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerLevel)?;
    buf.write(world.get_or_err::<Level>(entity)?.0)?;
    buf.write(world.get_or_err::<Player>(entity)?.levelexp)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_money(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerMoney)?;
    buf.write(world.get_or_err::<Money>(entity)?.vals)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_life_status(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    toself: bool,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDeath)?;
    buf.write(*entity)?;
    buf.write(world.get_or_err::<DeathType>(entity)?)?;
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
pub fn send_action(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    toself: bool,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAction)?;
    buf.write(world.get_or_err::<Dir>(entity)?.0)?;
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
pub fn send_pvp(world: &mut World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPvp)?;
    buf.write(world.get_or_err::<Player>(entity)?.pvpon)?;
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
pub fn send_pk(world: &mut World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
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
    entity: &Entity,
    msg: String,
    head: String,
    chan: MessageChannel,
    id: Option<usize>,
) -> Result<()> {
    let access = world.get_or_err::<UserAccess>(entity)?;

    match chan {
        MessageChannel::Map => DataTaskToken::MapChat(world.get_or_err::<Position>(entity)?.map)
            .add_task(storage, &MessagePacket::new(chan, head, msg, Some(access)))?,
        MessageChannel::Global => DataTaskToken::GlobalChat
            .add_task(storage, &MessagePacket::new(chan, head, msg, Some(access)))?,
        MessageChannel::Party | MessageChannel::Trade | MessageChannel::Help => {}
        MessageChannel::Private => {
            let mut buf = ByteBuffer::new_packet_with(msg.len() + head.len() + 32)?;

            buf.write(ServerPackets::ChatMsg)?;
            buf.write(1_u32)?;
            buf.write(chan)?;
            buf.write(head)?;
            buf.write(msg)?;
            buf.write(Some(access))?;
            buf.finish()?;

            if let Some(i) = id {
                send_to(storage, i, buf.clone())?;
            }
            send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)?;
        }
        MessageChannel::Guild => {}
        MessageChannel::Quest | MessageChannel::Npc => {
            let mut buf = ByteBuffer::new_packet_with(msg.len() + head.len() + 32)?;

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
pub fn send_openstorage(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(8)?;

    buf.write(ServerPackets::OpenStorage)?;
    buf.write(1_u32)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_openshop(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    shop_index: u16,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6)?;

    buf.write(ServerPackets::OpenShop)?;
    buf.write(shop_index)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_clearisusingtype(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6)?;

    buf.write(ServerPackets::ClearIsUsingType)?;
    buf.write(1_u16)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_updatetradeitem(
    world: &mut World,
    storage: &Storage,
    target_entity: &Entity,
    send_entity: &Entity,
    trade_slot: u16,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(8)?;

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
    target_entity: &Entity,
    send_entity: &Entity,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6)?;

    buf.write(ServerPackets::UpdateTradeMoney)?;
    buf.write(world.get::<&TradeMoney>(target_entity.0)?.vals)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(send_entity.0)?.id, buf)
}

#[inline]
pub fn send_inittrade(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(12)?;

    buf.write(ServerPackets::InitTrade)?;
    buf.write(*target_entity)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_tradestatus(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    my_status: &TradeStatus,
    their_status: &TradeStatus,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(12)?;

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
    entity: &Entity,
    target_entity: &Entity,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(12)?;

    buf.write(ServerPackets::TradeRequest)?;
    buf.write(*entity)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(target_entity.0)?.id, buf)
}

#[inline]
pub fn send_playitemsfx(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    item_index: u16,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(12)?;

    buf.write(ServerPackets::PlayItemSfx)?;
    buf.write(item_index)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0)?.id, buf)
}

#[inline]
pub fn send_floattextdamage(
    world: &mut World,
    storage: &Storage,
    pos: Position,
    damage: u16,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(12)?;

    buf.write(ServerPackets::FloatTextDamage)?;
    buf.write(damage)?;
    buf.write(pos)?;
    buf.finish()?;

    send_to_maps(world, storage, pos.map, buf, None)
}

#[inline]
pub fn send_floattextheal(
    world: &mut World,
    storage: &Storage,
    pos: Position,
    amount: u16,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(12)?;

    buf.write(ServerPackets::FloatTextHeal)?;
    buf.write(amount)?;
    buf.write(pos)?;
    buf.finish()?;

    send_to_maps(world, storage, pos.map, buf, None)
}
