use crate::{containers::Storage, gametypes::*, players::*, socket::*, tasks::*, maps::*};
use bytey::ByteBuffer;
use unwrap_helpers::*;
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

    let socket = world.get::<&Socket>(entity.0).expect("Could not find Socket");

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
    let map =
        &unwrap_or_return!(storage.maps.get(&position), Err(AscendingError::Unhandled)).borrow();
    if let Some(item) = map.itemids.get(&id) {
        let itemdata = world.get::<&MapItem>(item.0).expect("Could not get MapItem").item.clone();
        let itempos = world.get::<&MapItem>(item.0).expect("Could not get MapItem").pos.clone();

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
pub fn playerdata(world: &World, storage: &Storage, entity: &Entity) -> Option<ByteBuffer> {
    let mut buf = ByteBuffer::new_packet_with(512).ok()?;

    let data = world.entity(entity.0).expect("Could not get Entity");

    buf.write(ServerPackets::PlayerData).ok()?;
    buf.write(*entity).ok()?;
    buf.write(&data.get::<&Account>().expect("Could not find Account").name.clone()).ok()?;
    buf.write(*data.get::<&UserAccess>().expect("Could not find UserAccess")).ok()?;
    buf.write(data.get::<&Sprite>().expect("Could not find Sprite").id).ok()?;
    buf.write(*data.get::<&Position>().expect("Could not find Position")).ok()?;
    buf.write(data.get::<&Dir>().expect("Could not find Dir").0).ok()?;
    buf.write(data.get::<&Level>().expect("Could not find Level").0).ok()?;
    buf.write(data.get::<&Player>().expect("Could not find Player").levelexp).ok()?;
    buf.write(data.get::<&Vitals>().expect("Could not find Vitals").vital).ok()?;
    buf.write(data.get::<&Vitals>().expect("Could not find Vitals").vitalmax).ok()?;
    buf.write(data.get::<&Equipment>().expect("Could not find Equipment").items).ok()?;
    buf.write(*data.get::<&IsUsingType>().expect("Could not find IsUsingType")).ok()?;
    buf.write(data.get::<&Player>().expect("Could not find Player").resetcount).ok()?;
    buf.write(*data.get::<&DeathType>().expect("Could not find DeathType")).ok()?;
    buf.write(data.get::<&Hidden>().expect("Could not find Hidden").0).ok()?;
    buf.write(data.get::<&Attacking>().expect("Could not find Attacking").0).ok()?;
    buf.write(data.get::<&Player>().expect("Could not find Player").pvpon).ok()?;
    buf.write(data.get::<&Player>().expect("Could not find Player").pk).ok()?;
    buf.finish().ok()?;

    return Some(buf);
}

#[inline]
pub fn send_dir(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDir)?;
    buf.write(world.get::<&Dir>(entity.0).expect("Could not find Dir").0)?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map,
        buf, closure(toself, *entity));

    Ok(())
}

#[inline]
pub fn send_move(world: &World, storage: &Storage, entity: &Entity, warp: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(24)?;

    let pos = world.get::<&Position>(entity.0).expect("Could not find Position");

    buf.write(ServerPackets::PlayerMove)?;
    buf.write(*entity)?;
    buf.write(*pos)?;
    buf.write(world.get::<&Dir>(entity.0).expect("Could not find Dir").0)?;
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

pub fn send_data_remove(world: &World, storage: &Storage, id: u64, map: MapPosition, datatype: u8) -> Result<()> {
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

    let vitals = world.get::<&Vitals>(entity.0).expect("Could not find Vitals");

    buf.write(ServerPackets::PlayerVitals)?;
    buf.write(vitals.vital)?;
    buf.write(vitals.vitalmax)?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map, 
        buf, None);

    Ok(())
}

#[inline]
pub fn send_inv(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(6500)?;

    buf.write(ServerPackets::PlayerInv)?;
    buf.write(world.get::<&Inventory>(entity.0).expect("Could not find Inventory").items)?;
    buf.finish()?;

    send_to(storage, 
        world.get::<&Socket>(entity.0).expect("Could not find Socket").id, buf);

    Ok(())
}

#[inline]
pub fn send_invslot(world: &World, storage: &Storage, entity: &Entity, id: usize) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(32)?;

    buf.write(ServerPackets::PlayerInvSlot)?;
    buf.write(id)?;
    buf.write(world.get::<&Inventory>(entity.0).expect("Could not find Inventory").items[id])?;
    buf.finish()?;

    send_to(storage, 
        world.get::<&Socket>(entity.0).expect("Could not find Socket").id, buf);

    Ok(())
}

#[inline]
pub fn send_attack(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAttack)?;
    buf.write(*entity)?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map, 
        buf, closure(toself, *entity));

    Ok(())
}

#[inline]
pub fn send_equipment(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerEquipment)?;
    buf.write(world.get::<&Equipment>(entity.0).expect("Could not find Equipment").items)?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map, 
        buf, None);

    Ok(())
}

#[inline]
pub fn send_level(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerLevel)?;
    buf.write(world.get::<&Level>(entity.0).expect("Could not find Level").0)?;
    buf.write(world.get::<&Player>(entity.0).expect("Could not find Player").levelexp)?;
    buf.finish()?;

    send_to(storage, 
        world.get::<&Socket>(entity.0).expect("Could not find Socket").id, buf);
    Ok(())
}

#[inline]
pub fn send_money(world: &World, storage: &Storage, entity: &Entity) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;

    buf.write(ServerPackets::PlayerMoney)?;
    buf.write(world.get::<&Money>(entity.0).expect("Could not find Money").vals)?;
    buf.finish()?;

    send_to(storage, 
        world.get::<&Socket>(entity.0).expect("Could not find Socket").id, buf);
    Ok(())
}

#[inline]
pub fn send_life_status(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerDeath)?;
    buf.write(*entity)?;
    buf.write(*world.get::<&DeathType>(entity.0).expect("Could not find DeathType"))?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map, 
        buf, closure(toself, *entity));
    Ok(())
}

#[inline]
pub fn send_action(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerAction)?;
    buf.write(world.get::<&Dir>(entity.0).expect("Could not find Dir").0)?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map, buf, closure(toself, *entity));
    Ok(())
}

#[inline]
pub fn send_pvp(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPvp)?;
    buf.write(world.get::<&Player>(entity.0).expect("Could not find Player").pvpon)?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map, 
        buf, closure(toself, *entity));
    Ok(())
}

#[inline]
pub fn send_pk(world: &World, storage: &Storage, entity: &Entity, toself: bool) -> Result<()> {
    let mut buf = ByteBuffer::new_packet_with(16)?;
    let closure = |toself, id| if toself { Some(id) } else { None };

    buf.write(ServerPackets::PlayerPk)?;
    buf.write(world.get::<&Player>(entity.0).expect("Could not find Player").pk)?;
    buf.finish()?;

    send_to_maps(world, storage, 
        world.get::<&Position>(entity.0).expect("Could not find Position").map, 
        buf, closure(toself, *entity));
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

    let access = world.get::<&UserAccess>(entity.0).expect("Could not find UserAccess");

    match chan {
        MessageChannel::Map => DataTaskToken::MapChat(
            world.get::<&Position>(entity.0).expect("Could not find Position").map).add_task(
            world, storage,
            &MessagePacket::new(chan, head, msg, Some(*access)),
        )?,
        MessageChannel::Global => DataTaskToken::GlobalChat.add_task(
            world, storage,
            &MessagePacket::new(chan, head, msg, Some(*access)),
        )?,
        MessageChannel::Party | MessageChannel::Trade | MessageChannel::Help => {}
        MessageChannel::Private => {
            let mut buf = ByteBuffer::new_packet_with(msg.len() + head.len() + 32)?;

            buf.write(ServerPackets::ChatMsg)?;
            buf.write(chan)?;
            buf.write(head)?;
            buf.write(msg)?;
            buf.write(*access)?;
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
            buf.write(*access)?;
            buf.finish()?;
            send_to(storage, 
                world.get::<&Socket>(entity.0).expect("Could not find Socket").id, buf);
        }
    }

    Ok(())
}
