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

    send_to(storage, socket_id, buf);
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

    send_to(storage, socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_loginok(storage: &Storage, socket_id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::LoginOk)?;
    buf.write(storage.time.borrow().hour)?;
    buf.write(storage.time.borrow().min)?;
    buf.finish()?;

    send_to(storage, socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_updatemap(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::UpdateMap)?;
    buf.finish()?;

    let socket = world.get_or_panic::<&Socket>(entity);

    send_to(storage, socket.id, buf);
    Ok(())
}

#[inline]
pub fn send_mapitem(
    world: &mut hecs::World,
    storage: &Storage,
    position: MapPosition,
    id: Entity,
    sendto: Option<usize>,
) -> Result<()> {
    let map = match storage.maps.get(&position) {
        Some(map) => map,
        None => return Err(AscendingError::Unhandled),
    }
    .borrow();
    if let Some(item) = map.itemids.get(&id) {
        let itemdata = world.get_or_panic::<MapItem>(item);
        let itempos = world.get_or_panic::<MapItem>(item).pos;

        let mut buf = ByteBuffer::new_packet_with(64)?;

        buf.write(ServerPackets::MapItems)?;
        buf.write(*item)?;
        buf.write(itemdata)?;
        buf.write(itempos)?;
        buf.finish()?;

        if let Some(socket_id) = sendto {
            send_to(storage, socket_id, buf);
        } else {
            send_to_maps(world, storage, position, buf, None);
        }
    }
    Ok(())
}

#[inline]
pub fn playerdata(world: &World, _storage: &Storage, entity: &Entity) -> Option<ByteBuffer> {
    let mut buf = ByteBuffer::new_packet_with(512).ok()?;

    buf.write(ServerPackets::PlayerData).ok()?;
    buf.write(*entity).ok()?;
    buf.write(world.get_or_panic::<&Account>(entity).username.clone())
        .ok()?;
    buf.write(world.get_or_panic::<UserAccess>(entity)).ok()?;
    buf.write(world.get_or_panic::<Sprite>(entity).id).ok()?;
    buf.write(world.get_or_panic::<Position>(entity)).ok()?;
    buf.write(world.get_or_panic::<Dir>(entity).0).ok()?;
    buf.write(world.get_or_panic::<Level>(entity).0).ok()?;
    buf.write(world.get_or_panic::<Player>(entity).levelexp)
        .ok()?;
    buf.write(world.get_or_panic::<Vitals>(entity).vital).ok()?;
    buf.write(world.get_or_panic::<Vitals>(entity).vitalmax)
        .ok()?;
    buf.write(world.get_or_panic::<&Equipment>(entity).items.clone())
        .ok()?;
    buf.write(world.get_or_panic::<IsUsingType>(entity)).ok()?;
    buf.write(world.get_or_panic::<Player>(entity).resetcount)
        .ok()?;
    buf.write(world.get_or_panic::<DeathType>(entity)).ok()?;
    buf.write(world.get_or_panic::<Hidden>(entity).0).ok()?;
    buf.write(world.get_or_panic::<Attacking>(entity).0).ok()?;
    buf.write(world.get_or_panic::<Player>(entity).pvpon).ok()?;
    buf.write(world.get_or_panic::<Player>(entity).pk).ok()?;
    buf.finish().ok()?;

    Some(buf)
}

#[inline]
pub fn send_dir(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDir)?;
    buf.write(world.get_or_panic::<Dir>(entity).0)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        closure(toself, *entity),
    );

    Ok(())
}

#[inline]
pub fn send_move(world: &World, storage: &Storage, entity: &Entity, warp: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    let pos = world.get_or_panic::<Position>(entity);

    buf.write(ServerPackets::PlayerMove)?;
    buf.write(*entity)?;
    buf.write(pos)?;
    buf.write(world.get_or_panic::<Dir>(entity).0)?;
    buf.write(warp)?;
    buf.finish()?;

    if warp {
        send_to_maps(world, storage, pos.map, buf, None);
    } else {
        send_to_maps(world, storage, pos.map, buf, Some(*entity));
    }

    Ok(())
}

pub fn send_data_remove_list(
    storage: &Storage,
    playerid: usize,
    remove: &[Entity],
    datatype: u8,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::Dataremovelist)?;
    buf.write(datatype)?;
    buf.write(remove.to_vec())?;
    buf.finish()?;

    send_to(storage, playerid, buf);

    Ok(())
}

pub fn send_data_remove(
    world: &World,
    storage: &Storage,
    id: u64,
    map: MapPosition,
    datatype: u8,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::Dataremove)?;
    buf.write(datatype)?;
    buf.write(id)?;
    buf.finish()?;

    send_to_maps(world, storage, map, buf, None);

    Ok(())
}

pub fn send_data_remove_all(world: &World, storage: &Storage, id: u64, datatype: u8) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    buf.write(ServerPackets::Dataremove)?;
    buf.write(datatype)?;
    buf.write(id)?;
    buf.finish()?;

    send_to_all(world, storage, buf);

    Ok(())
}

#[inline]
pub fn send_vitals(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    let vitals = world.get_or_panic::<Vitals>(entity);

    buf.write(ServerPackets::PlayerVitals)?;
    buf.write(vitals.vital)?;
    buf.write(vitals.vitalmax)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        None,
    );

    Ok(())
}

#[inline]
pub fn send_inv(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write(ServerPackets::PlayerInv)?;
    buf.write(world.get_or_panic::<&Inventory>(entity).items.clone())?;
    buf.finish()?;

    send_to(storage, world.get_or_panic::<&Socket>(entity).id, buf);

    Ok(())
}

#[inline]
pub fn send_invslot(world: &World, storage: &Storage, entity: &Entity, id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write(ServerPackets::PlayerInvSlot)?;
    buf.write(id)?;
    buf.write(world.get_or_panic::<&Inventory>(entity).items[id])?;
    buf.finish()?;

    send_to(storage, world.get_or_panic::<&Socket>(entity).id, buf);

    Ok(())
}

#[inline]
pub fn send_attack(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAttack)?;
    buf.write(*entity)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        closure(toself, *entity),
    );

    Ok(())
}

#[inline]
pub fn send_equipment(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerEquipment)?;
    buf.write(world.get_or_panic::<&Equipment>(entity).items.clone())?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        None,
    );

    Ok(())
}

#[inline]
pub fn send_level(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerLevel)?;
    buf.write(world.get_or_panic::<Level>(entity).0)?;
    buf.write(world.get_or_panic::<Player>(entity).levelexp)?;
    buf.finish()?;

    send_to(storage, world.get_or_panic::<&Socket>(entity).id, buf);
    Ok(())
}

#[inline]
pub fn send_money(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerMoney)?;
    buf.write(world.get_or_panic::<Money>(entity).vals)?;
    buf.finish()?;

    send_to(storage, world.get_or_panic::<&Socket>(entity).id, buf);
    Ok(())
}

#[inline]
pub fn send_life_status(
    world: &World,
    storage: &Storage,
    entity: &Entity,
    toself: bool,
) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDeath)?;
    buf.write(*entity)?;
    buf.write(world.get_or_panic::<DeathType>(entity))?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        closure(toself, *entity),
    );
    Ok(())
}

#[inline]
pub fn send_action(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAction)?;
    buf.write(world.get_or_panic::<Dir>(entity).0)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        closure(toself, *entity),
    );
    Ok(())
}

#[inline]
pub fn send_pvp(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPvp)?;
    buf.write(world.get_or_panic::<Player>(entity).pvpon)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        closure(toself, *entity),
    );
    Ok(())
}

#[inline]
pub fn send_pk(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPk)?;
    buf.write(world.get_or_panic::<Player>(entity).pk)?;
    buf.finish()?;

    send_to_maps(
        world,
        storage,
        world.get_or_panic::<Position>(entity).map,
        buf,
        closure(toself, *entity),
    );
    Ok(())
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
    let access = world.get_or_panic::<UserAccess>(entity);

    match chan {
        MessageChannel::Map => {
            DataTaskToken::MapChat(world.get_or_panic::<Position>(entity).map)
                .add_task(storage, &MessagePacket::new(chan, head, msg, Some(access)))?
        }
        MessageChannel::Global => DataTaskToken::GlobalChat
            .add_task(storage, &MessagePacket::new(chan, head, msg, Some(access)))?,
        MessageChannel::Party | MessageChannel::Trade | MessageChannel::Help => {}
        MessageChannel::Private => {
            let mut buf = ByteBuffer::new_packet_with(msg.len() + head.len() + 32)?;

            buf.write(ServerPackets::ChatMsg)?;
            buf.write(chan)?;
            buf.write(head)?;
            buf.write(msg)?;
            buf.write(access)?;
            buf.finish()?;

            if let Some(i) = id {
                send_to(storage, i, buf);
            }
        }
        MessageChannel::Guild => {}
        MessageChannel::Quest | MessageChannel::Npc => {
            let mut buf = ByteBuffer::new_packet_with(msg.len() + head.len() + 32)?;

            buf.write(ServerPackets::ChatMsg)?;
            buf.write(chan)?;
            buf.write(head)?;
            buf.write(msg)?;
            buf.write(access)?;
            buf.finish()?;
            send_to(storage, world.get_or_panic::<&Socket>(entity).id, buf);
        }
    }

    Ok(())
}
