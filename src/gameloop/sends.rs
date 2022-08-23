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

    buf.write::<u32>(ServerPackets::Alertmsg as u32)?;
    buf.write_str(&message)?;
    buf.write::<u8>(close_socket)?;
    buf.finish()?;

    send_to(world, socket_id, &buf);
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

    buf.write::<u32>(ServerPackets::Fltalert as u32)?;
    buf.write(&ftltype)?;
    buf.write_str(&message)?;
    buf.finish()?;

    send_to(world, socket_id, &buf);
    Ok(())
}

#[inline]
pub fn send_loginok(world: &Storage, socket_id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write::<u32>(ServerPackets::Loginok as u32)?;
    buf.write::<u32>(world.time.borrow().hour)?;
    buf.write::<u32>(world.time.borrow().min)?;
    buf.finish()?;

    send_to(world, socket_id, &buf);
    Ok(())
}

#[inline]
pub fn send_updatemap(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write::<u32>(ServerPackets::Updatemap as u32)?;
    buf.finish()?;

    send_to(world, user.socket_id, &buf);
    Ok(())
}

#[inline]
pub fn send_mapitem(
    world: &Storage,
    position: MapPosition,
    id: u64,
    sendto: Option<usize>,
) -> Result<()> {
    let map = &unwrap_or_return!(
        world.map_data.get(&position),
        Err(AscendingError::Unhandled)
    )
    .borrow();
    if let Some(item) = map.items.get(id as usize) {
        let mut buf = ByteBuffer::new_packet_with(64)?;
        buf.write::<u32>(ServerPackets::Mapitem as u32)?;
        buf.write(&position)?;
        buf.write::<u64>(id)?;
        buf.write(&item.item)?;
        buf.write(&item.pos)?;
        buf.finish()?;

        if let Some(socket_id) = sendto {
            send_to(world, socket_id, &buf);
        } else {
            send_to_maps(world, position, &buf, None);
        }
    }
    Ok(())
}

#[inline]
pub fn playerdata(world: &Storage, id: u64) -> Option<ByteBuffer> {
    let mut buf = ByteBuffer::new_packet_with(512).ok()?;

    if let Some(refplayer) = world.players.borrow().get(id as usize) {
        let player = refplayer.borrow();

        buf.write::<u32>(ServerPackets::Playerdata as u32).ok()?;
        buf.write::<u64>(id).ok()?;

        buf.write_str(&player.name).ok()?;
        buf.write(&player.access).ok()?;
        buf.write::<u8>(player.sprite).ok()?;
        buf.write(&player.e.pos).ok()?;
        buf.write::<u8>(player.e.dir).ok()?;
        buf.write::<i32>(player.e.level).ok()?;
        buf.write::<u64>(player.levelexp).ok()?;

        for i in 0..VITALS_MAX {
            buf.write::<i32>(player.e.vital[i]).ok()?;
            buf.write::<i32>(player.e.vitalmax[i]).ok()?;
        }

        for i in 0..EQUIPMENT_TYPE_MAX {
            buf.write(&player.equip[i]).ok()?;
        }

        buf.write(&player.using).ok()?;
        buf.write::<i16>(player.resetcount).ok()?;
        buf.write(&player.e.life).ok()?;
        buf.write::<u8>(player.e.hidden as u8).ok()?;
        buf.write::<u8>(player.e.attacking as u8).ok()?;
        buf.write::<u8>(player.pvpon as u8).ok()?;
        buf.write::<u8>(player.pk as u8).ok()?;
        buf.finish().ok()?;

        return Some(buf);
    }

    None
}

#[inline]
pub fn send_dir(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write::<u32>(ServerPackets::Playerdir as u32)?;
    buf.write::<u8>(user.e.dir)?;
    buf.finish()?;
    send_to_maps(
        world,
        user.e.pos.map,
        &buf,
        closure(toself, user.e.get_id()),
    );

    Ok(())
}

#[inline]
pub fn send_move(world: &Storage, user: &Player, warp: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write::<u32>(ServerPackets::Playermove as u32)?;
    buf.write::<u64>(user.e.get_id() as u64)?;
    buf.write(&user.e.pos)?;
    buf.write::<u8>(user.e.dir)?;
    buf.write::<u8>(warp as u8)?;
    buf.finish()?;

    if warp {
        send_to_maps(world, user.e.pos.map, &buf, None);
    } else {
        send_to_maps(world, user.e.pos.map, &buf, Some(user.e.get_id()));
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

    buf.write::<u32>(ServerPackets::Playermapswap as u32)?;
    buf.write::<u64>(user.e.get_id() as u64)?;
    buf.write(&user.e.pos)?;
    buf.write::<u8>(user.e.dir)?;
    buf.write(&oldmap)?;
    buf.write::<u8>(warp as u8)?;
    buf.finish()?;

    if warp {
        send_to_maps(world, oldmap, &buf, None);
        send_to_maps(world, user.e.pos.map, &buf, None);
    } else {
        send_to_maps(world, oldmap, &buf, Some(user.e.get_id()));
        send_to_maps(world, user.e.pos.map, &buf, Some(user.e.get_id()));
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

    buf.write::<u32>(ServerPackets::Dataremovelist as u32)?;
    buf.write::<u8>(datatype)?;
    buf.write::<u64>(remove.len() as u64)?;

    for i in remove {
        buf.write::<u64>(*i as u64)?;
    }

    buf.finish()?;

    send_to(world, playerid, &buf);

    Ok(())
}

pub fn send_data_remove(world: &Storage, id: u64, map: MapPosition, datatype: u8) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write::<u32>(ServerPackets::Dataremove as u32)?;
    buf.write::<u8>(datatype)?;
    buf.write::<u64>(id)?;

    buf.finish()?;

    send_to_maps(world, map, &buf, None);

    Ok(())
}

pub fn send_data_remove_all(world: &Storage, id: u64, datatype: u8) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write::<u32>(ServerPackets::Dataremove as u32)?;
    buf.write::<u8>(datatype)?;
    buf.write::<u64>(id)?;

    buf.finish()?;

    send_to_all(world, &buf);

    Ok(())
}

#[inline]
pub fn send_vitals(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write::<u32>(ServerPackets::Playervitals as u32)?;
    for i in 0..VITALS_MAX {
        buf.write::<i32>(user.e.vital[i])?;
        buf.write::<i32>(user.e.vitalmax[i])?;
    }
    buf.finish()?;
    send_to_maps(world, user.e.pos.map, &buf, None);

    Ok(())
}

#[inline]
pub fn send_inv(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write::<u32>(ServerPackets::Playerinv as u32)?;
    for i in 0..MAX_INV {
        buf.write(&user.inv[i])?;
    }
    buf.finish()?;
    send_to(world, user.socket_id, &buf);

    Ok(())
}

#[inline]
pub fn send_invslot(world: &Storage, user: &Player, id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write::<u32>(ServerPackets::Playerinvslot as u32)?;
    buf.write::<u64>(id as u64)?;
    buf.write(&user.inv[id])?;
    buf.finish()?;
    send_to(world, user.socket_id, &buf);

    Ok(())
}

#[inline]
pub fn send_attack(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write::<u32>(ServerPackets::Playerattack as u32)?;
    buf.write::<u64>(user.e.get_id() as u64)?;
    buf.finish()?;
    send_to_maps(
        world,
        user.e.pos.map,
        &buf,
        closure(toself, user.e.get_id()),
    );

    Ok(())
}

#[inline]
pub fn send_equipment(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write::<u32>(ServerPackets::Playerequipment as u32)?;
    for i in 0..EQUIPMENT_TYPE_MAX {
        buf.write(&user.equip[i])?;
    }
    buf.finish()?;
    send_to_maps(world, user.e.pos.map, &buf, None);

    Ok(())
}

#[inline]
pub fn send_level(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write::<u32>(ServerPackets::Playerlevel as u32)?;
    buf.write::<i32>(user.e.level)?;
    buf.write::<u64>(user.levelexp)?;
    buf.finish()?;
    send_to(world, user.socket_id, &buf);

    Ok(())
}

#[inline]
pub fn send_money(world: &Storage, user: &Player) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write::<u32>(ServerPackets::Playermoney as u32)?;
    buf.write::<u64>(user.vals)?;
    buf.finish()?;
    send_to(world, user.socket_id, &buf);

    Ok(())
}

#[inline]
pub fn send_life_status(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write::<u32>(ServerPackets::Playerdeathstatus as u32)?;
    buf.write::<u64>(user.e.get_id() as u64)?;
    buf.write(&user.e.life)?;
    buf.finish()?;
    send_to_maps(
        world,
        user.e.pos.map,
        &buf,
        closure(toself, user.e.get_id()),
    );

    Ok(())
}

#[inline]
pub fn send_action(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write::<u32>(ServerPackets::Playerdir as u32)?;
    buf.write::<u8>(user.e.dir)?;
    buf.finish()?;
    send_to_maps(
        world,
        user.e.pos.map,
        &buf,
        closure(toself, user.e.get_id()),
    );

    Ok(())
}

#[inline]
pub fn send_pvp(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write::<u32>(ServerPackets::Playerdir as u32)?;
    buf.write::<u8>(user.pvpon as u8)?;
    buf.finish()?;
    send_to_maps(
        world,
        user.e.pos.map,
        &buf,
        closure(toself, user.e.get_id()),
    );

    Ok(())
}

#[inline]
pub fn send_pk(world: &Storage, user: &Player, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write::<u32>(ServerPackets::Playerdir as u32)?;
    buf.write::<u8>(user.pk as u8)?;
    buf.finish()?;
    send_to_maps(
        world,
        user.e.pos.map,
        &buf,
        closure(toself, user.e.get_id()),
    );

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

    buf.write::<u32>(ServerPackets::Playermsg as u32)?;
    buf.write(&chan)?;
    buf.write_str(&head)?;
    buf.write_str(&msg)?;
    buf.write(&user.access)?;
    buf.finish()?;

    match chan {
        MessageChannel::Map => send_to_maps(world, user.e.pos.map, &buf, None),
        MessageChannel::Global | MessageChannel::Trade | MessageChannel::Help => {
            send_to_all(world, &buf)
        }
        MessageChannel::Party => {}
        MessageChannel::Private => {
            if let Some(i) = id {
                send_to(world, i, &buf);
            }
        }
        MessageChannel::Guild => {}
        MessageChannel::Quest => send_to(world, user.socket_id, &buf),
        MessageChannel::Npc => send_to(world, user.socket_id, &buf),
    }

    Ok(())
}
