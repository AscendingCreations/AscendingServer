use crate::{containers::Storage, gametypes::*, players::*, socket::*};
use unwrap_helpers::*;

#[inline]
pub fn send_infomsg(
    world: &Storage,
    socket_id: usize,
    message: String,
    close_socket: u8,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(128)?;

    buf.write(ServerPackets::AlertMsg)?;
    buf.write(message)?;
    buf.write(close_socket)?;
    buf.finish()?;

    send_to(world, socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_fltalert(
    world: &Storage,
    socket_id: usize,
    message: String,
    ftltype: FtlType,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(128)?;

    buf.write(ServerPackets::FltAlert)?;
    buf.write(ftltype)?;
    buf.write(message)?;
    buf.finish()?;

    send_to(world, socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_loginok(world: &Storage, socket_id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::LoginOk)?;
    buf.write(world.time.borrow().hour)?;
    buf.write(world.time.borrow().min)?;
    buf.finish()?;

    send_to(world, socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_updatemap(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::UpdateMap)?;
    buf.finish()?;

    send_to(world, user.socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_mapitem(
    world: &Storage,
    position: MapPosition,
    id: u64,
    sendto: Option<usize>,
) -> Result<()> {
    let map =
        &unwrap_or_return!(world.maps.get(&position), Err(AscendingError::Unhandled)).borrow();
    if let Some(item) = map.items.get(id as usize) {
        let mut buf = ByteBuffer::new_packet_with(64)?;

        buf.write(ServerPackets::MapItems)?;
        buf.write(id)?;
        buf.write(item.item)?;
        buf.write(item.pos)?;
        buf.finish()?;

        if let Some(socket_id) = sendto {
            send_to(world, socket_id, buf);
        } else {
            send_to_maps(world, position, buf, None);
        }
    }
    Ok(())
}

#[inline]
pub fn playerdata(world: &Storage, id: u64) -> Option<ByteBuffer> {
    let mut buf = ByteBuffer::new_packet_with(512).ok()?;

    if let Some(refplayer) = world.players.borrow().get(id as usize) {
        let player = refplayer.borrow();

        buf.write(ServerPackets::PlayerData).ok()?;
        buf.write(id).ok()?;
        buf.write(&player.name).ok()?;
        buf.write(player.access).ok()?;
        buf.write(player.sprite).ok()?;
        buf.write(player.e.pos).ok()?;
        buf.write(player.e.dir).ok()?;
        buf.write(player.e.level).ok()?;
        buf.write(player.levelexp).ok()?;
        buf.write(player.e.vital).ok()?;
        buf.write(player.e.vitalmax).ok()?;
        buf.write(player.equip).ok()?;
        buf.write(player.using).ok()?;
        buf.write(player.resetcount).ok()?;
        buf.write(player.e.life).ok()?;
        buf.write(player.e.hidden).ok()?;
        buf.write(player.e.attacking).ok()?;
        buf.write(player.pvpon).ok()?;
        buf.write(player.pk).ok()?;
        buf.finish().ok()?;

        return Some(buf);
    }

    None
}

#[inline]
pub fn send_dir(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDir)?;
    buf.write(user.e.dir)?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, closure(toself, user.e.get_id()));

    Ok(())
}

#[inline]
pub fn send_move(world: &Storage, user: &Player, warp: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::PlayerMove)?;
    buf.write(user.e.get_id())?;
    buf.write(user.e.pos)?;
    buf.write(user.e.dir)?;
    buf.write(warp)?;
    buf.finish()?;

    if warp {
        send_to_maps(world, user.e.pos.map, buf, None);
    } else {
        send_to_maps(world, user.e.pos.map, buf, Some(user.e.get_id()));
    }

    Ok(())
}

#[inline]
pub fn send_mapswitch(
    world: &Storage,
    user: &Player,
    oldmap: MapPosition,
    warp: bool,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::PlayerMapSwap)?;
    buf.write(user.e.get_id())?;
    buf.write(user.e.pos)?;
    buf.write(user.e.dir)?;
    buf.write(oldmap)?;
    buf.write(warp)?;
    buf.finish()?;

    if warp {
        send_to_maps(world, oldmap, buf.clone(), None);
        send_to_maps(world, user.e.pos.map, buf, None);
    } else {
        send_to_maps(world, oldmap, buf.clone(), Some(user.e.get_id()));
        send_to_maps(world, user.e.pos.map, buf, Some(user.e.get_id()));
    }

    Ok(())
}

pub fn send_data_remove_list(
    world: &Storage,
    playerid: usize,
    remove: &[u64],
    datatype: u8,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::Dataremovelist)?;
    buf.write(datatype)?;
    buf.write(remove.to_vec())?;
    buf.finish()?;

    send_to(world, playerid, buf);

    Ok(())
}

pub fn send_data_remove(world: &Storage, id: u64, map: MapPosition, datatype: u8) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::Dataremove)?;
    buf.write(datatype)?;
    buf.write(id)?;
    buf.finish()?;

    send_to_maps(world, map, buf, None);

    Ok(())
}

pub fn send_data_remove_all(world: &Storage, id: u64, datatype: u8) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::Dataremove)?;
    buf.write(datatype)?;
    buf.write(id)?;
    buf.finish()?;

    send_to_all(world, buf);

    Ok(())
}

#[inline]
pub fn send_vitals(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write(ServerPackets::PlayerVitals)?;
    buf.write(user.e.vital)?;
    buf.write(user.e.vitalmax)?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, None);

    Ok(())
}

#[inline]
pub fn send_inv(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write(ServerPackets::PlayerInv)?;
    buf.write(user.inv)?;
    buf.finish()?;

    send_to(world, user.socket_id, buf);

    Ok(())
}

#[inline]
pub fn send_invslot(world: &Storage, user: &Player, id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write(ServerPackets::PlayerInvSlot)?;
    buf.write(id)?;
    buf.write(user.inv[id])?;
    buf.finish()?;

    send_to(world, user.socket_id, buf);

    Ok(())
}

#[inline]
pub fn send_attack(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAttack)?;
    buf.write(user.e.get_id())?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, closure(toself, user.e.get_id()));

    Ok(())
}

#[inline]
pub fn send_equipment(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerEquipment)?;
    buf.write(user.equip)?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, None);

    Ok(())
}

#[inline]
pub fn send_level(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerLevel)?;
    buf.write(user.e.level)?;
    buf.write(user.levelexp)?;
    buf.finish()?;

    send_to(world, user.socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_money(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerMoney)?;
    buf.write(user.vals)?;
    buf.finish()?;

    send_to(world, user.socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_life_status(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDeath)?;
    buf.write(user.e.get_id())?;
    buf.write(user.e.life)?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, closure(toself, user.e.get_id()));
    Ok(())
}

#[inline]
pub fn send_action(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAction)?;
    buf.write(user.e.dir)?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, closure(toself, user.e.get_id()));
    Ok(())
}

#[inline]
pub fn send_pvp(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPvp)?;
    buf.write(user.pvpon)?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, closure(toself, user.e.get_id()));
    Ok(())
}

#[inline]
pub fn send_pk(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPk)?;
    buf.write(user.pk)?;
    buf.finish()?;

    send_to_maps(world, user.e.pos.map, buf, closure(toself, user.e.get_id()));
    Ok(())
}

#[inline]
pub fn send_message(
    world: &Storage,
    user: &Player,
    msg: String,
    head: String,
    chan: MessageChannel,
    id: Option<usize>,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(msg.len() + head.len() + 32)?;

    buf.write(ServerPackets::ChatMsg)?;
    buf.write(chan)?;
    buf.write(head)?;
    buf.write(msg)?;
    buf.write(user.access)?;
    buf.finish()?;

    match chan {
        MessageChannel::Map => send_to_maps(world, user.e.pos.map, buf, None),
        MessageChannel::Global => send_to_all(world, buf),
        MessageChannel::Party | MessageChannel::Trade | MessageChannel::Help => {}
        MessageChannel::Private => {
            if let Some(i) = id {
                send_to(world, i, buf);
            }
        }
        MessageChannel::Guild => {}
        MessageChannel::Quest => send_to(world, user.socket_id, buf),
        MessageChannel::Npc => send_to(world, user.socket_id, buf),
    }

    Ok(())
}
