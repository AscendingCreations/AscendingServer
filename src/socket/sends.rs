use std::ops::Range;

use mio::Token;

use crate::{
    containers::{Entity, GlobalKey, Storage, TradeStatus, World},
    gametypes::*,
    socket::*,
    tasks::*,
};

#[inline]
pub fn send_infomsg(
    storage: &Storage,
    socket_id: Token,
    message: String,
    close_socket: u8,
) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::AlertMsg)?;
    buf.write(message)?;
    buf.write(close_socket)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_fltalert(
    storage: &Storage,
    socket_id: Token,
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
pub fn send_loginok(storage: &Storage, socket_id: Token) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::LoginOk)?;
    buf.write(storage.time.borrow().hour)?;
    buf.write(storage.time.borrow().min)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_myindex(storage: &Storage, socket_id: Token, entity: GlobalKey) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::MyIndex)?;
    buf.write(entity)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

pub fn send_move_ok(storage: &Storage, socket_id: Token, move_ok: bool) -> Result<()> {
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
    socket_id: Token,
    entity: GlobalKey,
) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerData)?;
        buf.write(&data.account.username)?;
        buf.write(data.user_access)?;
        buf.write(data.movement.dir)?;
        buf.write(&data.equipment)?;
        buf.write(data.combat.level)?;
        buf.write(data.combat.death_type)?;
        buf.write(data.combat.physical.damage)?;
        buf.write(data.combat.physical.defense)?;
        buf.write(data.movement.pos)?;
        buf.write(data.general.pk)?;
        buf.write(data.general.pvpon)?;
        buf.write(data.sprite.id as u8)?;
        buf.write(data.combat.vitals.vital)?;
        buf.write(data.combat.vitals.vitalmax)?;
        buf.finish()?;

        send_to(storage, socket_id, buf)?;
    }
    Ok(())
}

pub fn send_codes(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    code: String,
    handshake: String,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.tls_id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::HandShake)?;
    buf.write(&code)?;
    buf.write(&handshake)?;
    buf.finish()?;

    // Once the codes are Sent we need to set this to unencrypted mode as the client will be un unencrypted mode.
    send_to(storage, socket_id, buf)
}

pub fn send_ping(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OnlineCheck)?;
    buf.write(0u64)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_inv(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerInv)?;
        buf.write(&data.inventory.items)?;
        buf.finish()?;

        send_to(storage, data.socket.id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_invslot(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    id: usize,
) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerInvSlot)?;
        buf.write(id)?;
        buf.write(data.inventory.items[id])?;
        buf.finish()?;

        send_to(storage, data.socket.id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_storage(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    range: Range<usize>,
) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerStorage)?;
        buf.write(range.clone())?;
        buf.write(&data.storage.items[range])?;
        buf.finish()?;

        send_to(storage, data.socket.id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_storageslot(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    id: usize,
) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerStorageSlot)?;
        buf.write(id)?;
        buf.write(data.storage.items[id])?;
        buf.finish()?;

        send_to(storage, data.socket.id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_equipment(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerEquipment)?;
        buf.write(entity)?;
        buf.write(data.equipment.clone())?;
        buf.finish()?;

        send_to_maps(world, storage, data.movement.pos.map, buf, None)?;
    }
    Ok(())
}

#[inline]
pub fn send_level(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerLevel)?;
        buf.write(data.combat.level)?;
        buf.write(data.general.levelexp)?;
        buf.finish()?;

        send_to(storage, data.socket.id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_money(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::PlayerMoney)?;
        buf.write(data.money.vals)?;
        buf.finish()?;

        send_to(storage, data.socket.id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_pk(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    toself: bool,
) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;
        let closure = |toself, id| if toself { Some(id) } else { None };

        buf.write(ServerPackets::PlayerPk)?;
        buf.write(data.general.pk)?;
        buf.finish()?;

        send_to_maps(
            world,
            storage,
            data.movement.pos.map,
            buf,
            closure(toself, entity),
        )?;
    }
    Ok(())
}

#[inline]
pub fn send_message(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    msg: String,
    head: String,
    chan: MessageChannel,
    id: Option<Token>,
) -> Result<()> {
    if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        let data = data.try_lock()?;

        let access = data.user_access;

        match chan {
            MessageChannel::Map => DataTaskToken::MapChat(data.movement.pos.map)
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

                if let Some(token) = id {
                    send_to(storage, token, buf.try_clone()?)?;
                }
                send_to(storage, data.socket.id, buf)?;
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
                send_to(storage, data.socket.id, buf)?;
            }
        }
    }

    Ok(())
}

#[inline]
pub fn send_openstorage(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OpenStorage)?;
    buf.write(1_u32)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_openshop(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    shop_index: u16,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::OpenShop)?;
    buf.write(shop_index)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_clearisusingtype(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::ClearIsUsingType)?;
    buf.write(1_u16)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_updatetradeitem(
    world: &mut World,
    storage: &Storage,
    target_entity: GlobalKey,
    send_entity: GlobalKey,
    trade_slot: u16,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(send_entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    if let Some(Entity::Player(data)) = world.get_opt_entity(target_entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::UpdateTradeItem)?;
        buf.write(target_entity == send_entity)?;
        buf.write(trade_slot)?;
        buf.write(data.trade_item.items[trade_slot as usize])?;
        buf.finish()?;

        send_to(storage, socket_id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_updatetrademoney(
    world: &mut World,
    storage: &Storage,
    target_entity: GlobalKey,
    send_entity: GlobalKey,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(send_entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    if let Some(Entity::Player(data)) = world.get_opt_entity(target_entity) {
        let data = data.try_lock()?;

        let mut buf = MByteBuffer::new_packet()?;

        buf.write(ServerPackets::UpdateTradeMoney)?;
        buf.write(data.trade_money.vals)?;
        buf.finish()?;

        send_to(storage, socket_id, buf)?;
    }
    Ok(())
}

#[inline]
pub fn send_inittrade(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::InitTrade)?;
    buf.write(target_entity)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_tradestatus(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    my_status: &TradeStatus,
    their_status: &TradeStatus,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::TradeStatus)?;
    buf.write(*my_status)?;
    buf.write(*their_status)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_traderequest(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(target_entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::TradeRequest)?;
    buf.write(entity)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_playitemsfx(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    item_index: u16,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::PlayItemSfx)?;
    buf.write(item_index)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

#[inline]
pub fn send_gameping(storage: &Storage, socket_id: Token) -> Result<()> {
    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::Ping)?;
    buf.write(0u64)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}

pub fn send_tls_codes(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    code: String,
    handshake: String,
) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.tls_id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::TlsHandShake)?;
    buf.write(&code)?;
    buf.write(&handshake)?;
    buf.finish()?;

    // Once the codes are Sent we need to set this to unencrypted mode as the client will be un unencrypted mode.
    send_to(storage, socket_id, buf)
}

pub fn send_clear_data(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    let socket_id = if let Some(Entity::Player(data)) = world.get_opt_entity(entity) {
        data.try_lock()?.socket.tls_id
    } else {
        return Ok(());
    };

    let mut buf = MByteBuffer::new_packet()?;

    buf.write(ServerPackets::ClearData)?;
    buf.write(0u32)?;
    buf.finish()?;

    send_to(storage, socket_id, buf)
}
