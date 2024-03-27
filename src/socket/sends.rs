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
    let mut buf = ByteBuffer::new_packet_with(12)?;

    buf.write(ServerPackets::LoginOk)?;
    buf.write(storage.time.borrow().hour)?;
    buf.write(storage.time.borrow().min)?;
    buf.finish()?;

    send_to(storage, socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_myindex(storage: &Storage, socket_id: usize, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(8)?;

    buf.write(ServerPackets::MyIndex)?;
    buf.write(*entity)?;
    buf.finish()?;

    send_to(storage, socket_id, buf);
    Ok(())
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
    buf.write(world.get::<&Account>(entity.0).unwrap().username.clone())?;
    buf.write(world.cloned_get_or_panic::<UserAccess>(entity))?;
    buf.write(world.get_or_panic::<Dir>(entity).0)?;
    let equipment = world.cloned_get_or_panic::<Equipment>(entity);
    buf.write(equipment)?;
    buf.write(world.get_or_panic::<Hidden>(entity).0)?;
    buf.write(world.get_or_panic::<Level>(entity).0)?;
    buf.write(world.cloned_get_or_panic::<DeathType>(entity))?;
    buf.write(world.get_or_panic::<Physical>(entity).damage)?;
    buf.write(world.get_or_panic::<Physical>(entity).defense)?;
    buf.write(world.cloned_get_or_panic::<Position>(entity))?;
    buf.write(world.get_or_panic::<Player>(entity).pk)?;
    buf.write(world.get_or_panic::<Player>(entity).pvpon)?;
    buf.write(world.get_or_panic::<Sprite>(entity).id as u8)?;
    buf.write(world.get_or_panic::<Vitals>(entity).vital)?;
    buf.write(world.get_or_panic::<Vitals>(entity).vitalmax)?;
    buf.finish()?;

    send_to(storage, socket_id, buf);
    Ok(())
}

#[inline]
pub fn send_updatemap(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::UpdateMap)?;
    buf.finish()?;

    let id: usize = world.get::<&Socket>(entity.0)?.id;

    send_to(storage, id, buf);
    Ok(())
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
        None => return Err(AscendingError::Unhandled),
    }
    .borrow();
    if let Some(item) = map.itemids.get(&id) {
        let itemdata = world.get_or_panic::<MapItem>(item);

        let mut buf = ByteBuffer::new_packet_with(64)?;

        buf.write(ServerPackets::MapItems)?;
        buf.write(*item)?;
        buf.write(itemdata)?;
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
pub fn send_dir(world: &mut World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
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
        send_to_maps(world, storage, sendpos.map, buf, Some(*entity));
    } else {
        send_to_maps(world, storage, pos.map, buf, Some(*entity));
    }

    Ok(())
}

pub fn send_warp(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    let pos = world.get_or_panic::<Position>(entity);

    buf.write(ServerPackets::PlayerWarp)?;
    buf.write(*entity)?;
    buf.write(pos)?;
    buf.finish()?;

    send_to_maps(world, storage, pos.map, buf, None);

    Ok(())
}

pub fn send_data_remove_list(storage: &Storage, socket_id: usize, remove: &[Entity]) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(PACKET_DATA_LIMIT - 8)?;

    buf.write(ServerPackets::Dataremovelist)?;
    buf.write(remove.to_vec())?;
    buf.finish()?;

    send_to(storage, socket_id, buf);

    Ok(())
}

#[inline]
pub fn send_vitals(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
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
pub fn send_inv(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write(ServerPackets::PlayerInv)?;
    buf.write(world.cloned_get_or_panic::<Inventory>(entity).items.clone())?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);

    Ok(())
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
    buf.write(world.cloned_get_or_panic::<Inventory>(entity).items[id])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);

    Ok(())
}

#[inline]
pub fn send_storage(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write(ServerPackets::PlayerStorage)?;
    buf.write(
        world
            .cloned_get_or_panic::<PlayerStorage>(entity)
            .items
            .clone(),
    )?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);

    Ok(())
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
    buf.write(world.cloned_get_or_panic::<PlayerStorage>(entity).items[id])?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);

    Ok(())
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
        world.get_or_panic::<Position>(entity).map,
        buf,
        closure(toself, *entity),
    );

    Ok(())
}

#[inline]
pub fn send_equipment(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerEquipment)?;
    buf.write(world.cloned_get_or_panic::<Equipment>(entity).items.clone())?;
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
pub fn send_level(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerLevel)?;
    buf.write(world.get_or_panic::<Level>(entity).0)?;
    buf.write(world.get_or_panic::<Player>(entity).levelexp)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);
    Ok(())
}

#[inline]
pub fn send_money(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerMoney)?;
    buf.write(world.get_or_panic::<Money>(entity).vals)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);
    Ok(())
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
pub fn send_action(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    toself: bool,
) -> Result<()> {
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
pub fn send_pvp(world: &mut World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
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
pub fn send_pk(world: &mut World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
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
            buf.write(1_u32)?;
            buf.write(chan)?;
            buf.write(head)?;
            buf.write(msg)?;
            buf.write(Some(access))?;
            buf.finish()?;

            if let Some(i) = id {
                send_to(storage, i, buf);
            }
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
            send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);
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

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);
    Ok(())
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

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);
    Ok(())
}

#[inline]
pub fn send_clearisusingtype(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6)?;

    buf.write(ServerPackets::ClearIsUsingType)?;
    buf.write(1_u16)?;
    buf.finish()?;

    send_to(storage, world.get::<&Socket>(entity.0).unwrap().id, buf);
    Ok(())
}