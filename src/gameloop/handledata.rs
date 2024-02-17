use crate::{
    containers::Storage, gameloop::*, gametypes::*, maps::*, players::*, sql::*, tasks::*,
};
use bytey::ByteBuffer;
use chrono::Duration;
use phf::phf_map;
use regex::Regex;
use unwrap_helpers::*;

type PacketFunction = fn(&mut hecs::World, &Storage, &mut ByteBuffer, &Entity) -> Result<()>;

static PACKET_MAP: phf::Map<u32, PacketFunction> = phf_map! {
    0u32 => handle_register,
    1u32 => handle_move,
    2u32 => handle_move,
    3u32 => handle_move,
};

pub fn handle_data(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    let id: u32 = data.read()?;

    let onlinetype = *world.get::<&OnlineType>(entity.0).expect("Could not find OnlineType");

    match onlinetype {
        OnlineType::Online => {
            if id <= 1 {
                return Err(AscendingError::MultiLogin);
            }
        }
        OnlineType::Accepted => {
            if id > 1 {
                return Err(AscendingError::PacketManipulation { name: "".into() });
            }
        }
        OnlineType::None => {
            return Err(AscendingError::PacketManipulation { name: "".into() });
        }
    }

    let fun = unwrap_or_return!(PACKET_MAP.get(&id), Err(AscendingError::InvalidPacket));
    fun(world, storage, data, entity)
}

fn handle_register(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let email = data.read::<String>()?;
    let name = data.read::<String>()?;
    let sprite: u8 = data.read()?;
    let hair: u8 = data.read()?;

    let socket_id = world.get::<&Socket>(entity.0).expect("Could not find Socket").id.clone();

    //if let Ok(query) = world.entity(entity.0) {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        let email_regex = Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )?;

        if !username.chars().all(is_name_acceptable)
            || !password.chars().all(is_password_acceptable)
            || !name.chars().all(is_name_acceptable)
        {
            return send_infomsg(
                storage,
                socket_id,
                "Username, Name, or Password contains unaccepted Characters".into(),
                0,
            );
        }

        if username.len() >= 64 || name.len() >= 64 {
            return send_infomsg(
                storage,
                socket_id,
                "Username or Name has too many Characters, 64 Characters Max".into(),
                0,
            );
        }

        if password.len() >= 128 {
            return send_infomsg(
                storage,
                socket_id,
                "Password has too many Characters, 128 Characters Max".into(),
                0,
            );
        }

        if !email_regex.is_match(&email) || hair >= 8 || sprite >= 6 {
            return send_infomsg(
                storage,
                socket_id,
                "Email must be an actual email.".into(),
                0,
            );
        }

        match check_existance(&mut storage.pgconn.borrow_mut(), &username, &name, &email) {
            Ok(i) => match i {
                0 => {}
                1 => {
                    return send_infomsg(
                        storage,
                        socket_id,
                        "Username Exists. Please try Another.".into(),
                        0,
                    )
                }
                2 => {
                    return send_infomsg(
                        storage,
                        socket_id,
                        "Character Name Exists. Please try Another.".into(),
                        0,
                    )
                }
                3 => {
                    return send_infomsg(
                        storage,
                        socket_id,
                        "Email Already Exists. Please Try Another.".into(),
                        0,
                    )
                }
                _ => return Err(AscendingError::RegisterFail),
            },
            Err(_) => return Err(AscendingError::UserNotFound),
        }

        if let mut account = world.get::<&mut Account>(p.0).expect("Could not find Account") 
            { account.name = name };
        if let mut player_sprite = world.get::<&mut Sprite>(p.0).expect("Could not find Sprite") 
            { player_sprite.id = sprite as u32 };

        if new_player(
            &mut storage.pgconn.borrow_mut(),
            world,
            entity,
            username,
            password,
            email,
        )
        .is_err()
        {
            return send_infomsg(
                storage,
                socket_id,
                "There was an Issue Creating the player account. Please Contact Support.".into(),
                0,
            );
        }

        return send_infomsg(storage, socket_id,
             "Account Was Created. Please wait for the Verification code sent to your email before logging in.".into(), 1);
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_login(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;

    let socket_id = world.get::<&Socket>(entity.0).expect("Could not find Socket").id.clone();

    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if username.len() >= 64 || password.len() >= 128 {
            return send_infomsg(
                storage,
                socket_id,
                "Account does not Exist or Password is not Correct.".into(),
                0,
            );
        }

        let id = find_player(&mut storage.pgconn.borrow_mut(), &username, &password)?;
        let id = unwrap_or_return!(
            id,
            send_infomsg(
                storage,
                socket_id,
                "Account does not Exist or Password is not Correct.".into(),
                1,
            )
        );

        if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
            return send_infomsg(
                storage,
                socket_id,
                "Client needs to be updated.".into(),
                1,
            );
        }

        if let mut account = world.get::<&mut Account>(p.0).expect("Could not find Account")
            { account.id = id };

        if let Err(_e) = load_player(storage, &mut storage.pgconn.borrow_mut(), world, entity) {
            return send_infomsg(storage, 
                socket_id,
                "Error Loading User.".into(), 1);
        }

        send_loginok(storage, socket_id)?;

        //joingame(index);
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_move(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Ok(query) = world.entity(entity.0) {
        if !query.get::<&DeathType>().expect("Could not find DeathType").is_alive()
            || query.get::<&IsUsingType>().expect("Could not find IsUsingType").inuse()
            || query.get::<&Stunned>().expect("Could not find Stunned").0
        {
            return Ok(());
        }

        let dir: u8 = data.read()?;
        let data_pos: Position = data.read()?;

        if storage.bases.maps.get(&data_pos.map).is_none() || dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        let pos = *query.get::<&Position>().expect("Could not find Socket");

        if data_pos != pos {
            player_warp(world, storage, entity, &data_pos, dir);
            return Ok(());
        }

        player_movement(world, storage, entity, dir);
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_dir(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Ok(query) = world.entity(entity.0) {
        if !query.get::<&DeathType>().expect("Could not find DeathType").is_alive()
            || query.get::<&IsUsingType>().expect("Could not find IsUsingType").inuse()
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        if let mut query_dir = query.get::<&mut Dir>().expect("Could not find Dir") 
            { query_dir.0 = dir };

        send_dir(world, storage, entity, true)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_attack(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Ok(query) = world.entity(entity.0) {
        if !query.get::<&DeathType>().expect("Could not find DeathType").is_alive()
            || query.get::<&IsUsingType>().expect("Could not find IsUsingType").inuse()
            || query.get::<&Attacking>().expect("Could not find Attacking").0
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let _id = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        if let mut query_dir = query.get::<&mut Dir>().expect("Could not find Dir") {
            if query_dir.0 != dir {
                query_dir.0 = dir;
                send_dir(world, storage, entity, true)?;
            }
        };

        //TODO Add Attack funciton call here for player attacks
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_useitem(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Ok(query) = world.entity(entity.0) {
        if !query.get::<&DeathType>().expect("Could not find DeathType").is_alive()
            || query.get::<&IsUsingType>().expect("Could not find IsUsingType").inuse()
            || query.get::<&Attacking>().expect("Could not find Attacking").0
            || query.get::<&Stunned>().expect("Could not find Stunned").0
            || query.get::<&PlayerItemTimer>().expect("Could not find PlayerItemTimer").itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let _slot = data.read::<u16>()?;
        let _targettype = data.read::<u8>()?;

        if let mut itemtimer = query.get::<&mut PlayerItemTimer>().expect("Could not find PlayerItemTimer") 
            { itemtimer.itemtimer = *storage.gettick.borrow() + Duration::milliseconds(250); };

        //TODO useitem();
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_unequip(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get::<&DeathType>(p.0).expect("Could not find DeathType").is_alive()
            || world.get::<&IsUsingType>(p.0).expect("Could not find IsUsingType").inuse()
            || world.get::<&Attacking>(p.0).expect("Could not find Attacking").0
            || world.get::<&Stunned>(p.0).expect("Could not find Stunned").0
            || world.get::<&PlayerItemTimer>(p.0).expect("Could not find PlayerItemTimer").itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= EQUIPMENT_TYPE_MAX || world.get::<&Equipment>(p.0).expect("Could not find Equipment").items[slot].val == 0 {
            return Ok(());
        }

        let mut item = world.get::<&Equipment>(p.0).expect("Could not find Equipment").items[slot];
        let entity_data = entity.clone();
        let rem = give_item(world, storage, &entity_data, &mut item);

        if rem > 0 {
            return send_fltalert(
                storage,
                world.get::<&Socket>(p.0).expect("Could not find Socket").id,
                "Could not unequiped. No inventory space.".into(),
                FtlType::Error,
            );
        }

        if let mut equipment = world.get::<&mut Equipment>(p.0).expect("Could not find Equipment") 
            { equipment.items[slot] = item };

        let _ = update_equipment(&mut storage.pgconn.borrow_mut(), world, entity, slot);
        //TODO calculatestats();
        return send_equipment(world, storage, entity);
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_switchinvslot(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get::<&DeathType>(p.0).expect("Could not find DeathType").is_alive()
            || world.get::<&IsUsingType>(p.0).expect("Could not find IsUsingType").inuse()
            || world.get::<&Attacking>(p.0).expect("Could not find Attacking").0
            || world.get::<&Stunned>(p.0).expect("Could not find Stunned").0
            || world.get::<&PlayerItemTimer>(p.0).expect("Could not find PlayerItemTimer").itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if oldslot >= MAX_INV || newslot >= MAX_INV || 
            world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[oldslot].val == 0 {
            return Ok(());
        }

        let base1 = &storage.bases.items[
            world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[oldslot].num as usize];
        let invtype = get_inv_itemtype(base1);

        if get_inv_type(oldslot) != invtype || get_inv_type(newslot) != invtype {
            return Ok(());
        }

        let mut itemold = world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[oldslot];

        if world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[newslot].val > 0 {
            if world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[newslot].num == 
                world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[oldslot].num {
                set_inv_slot(world, storage, entity, &mut itemold, newslot, amount);
                if let mut inv = world.get::<&mut Inventory>(p.0).expect("Could not find Inventory") 
                    { inv.items[oldslot] = itemold };
                save_item(world, storage, entity, oldslot);
            } else if world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[oldslot].val == amount {
                let itemnew = world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[newslot];
                if let mut inv = world.get::<&mut Inventory>(p.0).expect("Could not find Inventory") 
                    { inv.items[newslot] = itemold };
                if let mut inv = world.get::<&mut Inventory>(p.0).expect("Could not find Inventory") 
                    { inv.items[oldslot] = itemnew };
                save_item(world, storage, entity, newslot);
                save_item(world, storage, entity, oldslot);
            } else {
                return send_fltalert(
                        storage,
                        world.get::<&Socket>(p.0).expect("Could not find Socket").id,
                        "Can not swap slots with a different containing items unless you swap everything."
                            .into(),
                        FtlType::Error
                    );
            }
        } else {
            set_inv_slot(world, storage, entity, &mut itemold, newslot, amount);
            if let mut inv = world.get::<&mut Inventory>(p.0).expect("Could not find Inventory") 
                { inv.items[oldslot] = itemold };
            save_item(world, storage, entity, oldslot);
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_pickup(world: &mut hecs::World, storage: &Storage, _data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        let mut remid: Option<(MapPosition, Entity)> = None;

        if !world.get::<&DeathType>(p.0).expect("Could not find DeathType").is_alive()
            || world.get::<&IsUsingType>(p.0).expect("Could not find IsUsingType").inuse()
            || world.get::<&Attacking>(p.0).expect("Could not find Attacking").0
            || world.get::<&Stunned>(p.0).expect("Could not find Stunned").0
            || world.get::<&PlayerMapTimer>(p.0).expect("Could not find PlayerMapTimer").mapitemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let mapids = get_maps_in_range(storage, &world.get::<&Position>(p.0).expect("Could not find Position"), 1);

        'remremove: for id in mapids {
            if let Some(x) = id.get() {
                let mut map = unwrap_continue!(storage.maps.get(&x)).borrow_mut();
                let _ = unwrap_continue!(storage.bases.maps.get(&world.get::<&Position>(p.0).expect("Could not find Position").map));
                let ids = map.itemids.clone();

                for i in ids {
                    let mut mapitems = world.get::<&mut MapItem>(i.0).expect("Could not get MapItem").clone();
                    if world.get::<&Position>(p.0).expect("Could not find Position")
                        .checkdistance(mapitems.pos.map_offset(id.into()))
                        <= 1
                    {
                        if mapitems.item.num == 0 {
                            let rem = player_give_vals(world, storage, entity, mapitems.item.val as u64);
                            mapitems.item.val = rem as u16;

                            if rem == 0 {
                                remid = Some((x, i));
                                break 'remremove;
                            }
                        } else {
                            let amount = mapitems.item.val;
                            let rem = give_item(world, storage, entity, &mut mapitems.item);
                            let item = &storage.bases.items[mapitems.item.num as usize];

                            if rem == 0 {
                                let st = match amount {
                                    0 | 1 => "",
                                    _ => "'s",
                                };

                                let _ = send_fltalert(
                                    storage,
                                    world.get::<&Socket>(p.0).expect("Could not find Socket").id,
                                    format!("You picked up {} {}{}.", amount, item.name, st),
                                    FtlType::Item,
                                );
                                remid = Some((x, i));
                            } else if rem < amount {
                                let st = match amount - rem {
                                    0 | 1 => "",
                                    _ => "'s",
                                };

                                let _ = send_fltalert(
                                        storage,
                                        world.get::<&Socket>(p.0).expect("Could not find Socket").id,
                                        format!("You picked up {} {}{}. Your inventory is Full so some items remain.", amount, item.name, st),
                                        FtlType::Item,
                                    );
                            }

                            break 'remremove;
                        }
                    }
                }
            }
        }

        if let Some(id) = remid {
            if let Some(map) = storage.maps.get(&id.0) {
                map.borrow_mut().remove_item(id.1); 
                let _ = DataTaskToken::ItemUnload(id.0).add_task(world, storage, &(id.1));
            }
        }

        if let mut mapitemtimer = world.get::<&mut PlayerMapTimer>(p.0).expect("Could not find PlayerMapTimer") 
            { mapitemtimer.mapitemtimer = *storage.gettick.borrow() + Duration::milliseconds(100); };
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_dropitem(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get::<&DeathType>(p.0).expect("Could not find DeathType").is_alive()
            || world.get::<&IsUsingType>(p.0).expect("Could not find IsUsingType").inuse()
            || world.get::<&Attacking>(p.0).expect("Could not find Attacking").0
            || world.get::<&Stunned>(p.0).expect("Could not find Stunned").0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if slot >= MAX_INV || 
        world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[slot].val == 0 ||
            amount == 0 {
            return Ok(());
        }

        //make sure it exists first.
        let _ = unwrap_or_return!(
            storage.bases.maps.get(&world.get::<&Position>(p.0).expect("Could not find Position").map),
            Err(AscendingError::Unhandled)
        );

        match get_inv_type(slot) {
            InvType::Quest | InvType::Key => {
                return send_fltalert(
                    storage,
                    world.get::<&Socket>(p.0).expect("Could not find Socket").id,
                    "You can not drop key or Quest items.".into(),
                    FtlType::Error,
                );
            }
            _ => {}
        }

        let mut mapitem = MapItem::new(0);

        mapitem.item = world.get::<&Inventory>(p.0).expect("Could not find Inventory").items[slot];
        mapitem.despawn = match *world.get::<&UserAccess>(p.0).expect("Could not find UserAccess") {
            UserAccess::Admin => None,
            _ => Some(*storage.gettick.borrow() + Duration::milliseconds(600000)),
        };
        mapitem.ownertimer = Some(*storage.gettick.borrow() + Duration::milliseconds(5000));
        mapitem.ownerid = world.get::<&Account>(p.0).expect("Could not find Account").id;
        mapitem.pos = *world.get::<&Position>(p.0).expect("Could not find Position");

        let leftover = take_itemslot(world, storage, entity, slot, amount);
        mapitem.item.val -= leftover;
        let mut map = unwrap_or_return!(
            storage.maps.get(&world.get::<&Position>(p.0).expect("Could not find Position").map),
            Err(AscendingError::Unhandled)
        )
        .borrow_mut();

        let id = map.add_mapitem(world, mapitem.clone());
        let pos = *world.get::<&Position>(p.0).expect("Could not find Position").clone();
        let _ = DataTaskToken::ItemLoad(pos.map).add_task(
            world,
            storage,
            &MapItemPacket::new(
                id,
                mapitem.pos,
                mapitem.item,
                Some(mapitem.ownerid),
            ),
        );
        
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_deleteitem(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Ok(query) = world.entity(entity.0) {
        if !query.get::<&DeathType>().expect("Could not find DeathType").is_alive()
            || query.get::<&IsUsingType>().expect("Could not find IsUsingType").inuse()
            || query.get::<&Attacking>().expect("Could not find Attacking").0
            || query.get::<&Stunned>().expect("Could not find Stunned").0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= MAX_INV || query.get::<&Inventory>().expect("Could not find Inventory").items[slot].val == 0 {
            return Ok(());
        }

        match get_inv_type(slot) {
            InvType::Quest | InvType::Key => {
                return send_fltalert(
                    storage,
                    query.get::<&Socket>().expect("Could not find Socket").id,
                    "You can not delete key or Quest items.".into(),
                    FtlType::Error,
                );
            }
            _ => {}
        }
        let val = query.get::<&Inventory>().expect("Could not find Inventory").items[slot].val;
        let _ = take_itemslot(world, storage, entity, slot, val);

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_message(world: &mut hecs::World, storage: &Storage, data: &mut ByteBuffer, entity: &Entity) -> Result<()> {
    if let Ok(query) = world.entity(entity.0) {
        let mut usersocket: Option<usize> = None;

        if !query.get::<&DeathType>().expect("Could not find DeathType").is_alive()
            || query.get::<&IsUsingType>().expect("Could not find IsUsingType").inuse()
            || query.get::<&Attacking>().expect("Could not find Attacking").0
            || query.get::<&Stunned>().expect("Could not find Stunned").0
        {
            return Ok(());
        }

        let channel: MessageChannel = data.read()?;

        let msg = data.read::<String>()?;
        let name = data.read::<String>()?;

        if msg.len() >= 256 {
            return send_fltalert(
                storage,
                query.get::<&Socket>().expect("Could not find Socket").id,
                "Your message is too long. (256 character limit)".into(),
                FtlType::Error,
            );
        }

        let head = match channel {
            MessageChannel::Map => format!("[Map] {}:", query.get::<&Account>().expect("Could not find Account").name),
            MessageChannel::Global => format!("[Global] {}:", query.get::<&Account>().expect("Could not find Account").name),
            MessageChannel::Trade => format!("[Trade] {}:", query.get::<&Account>().expect("Could not find Account").name),
            MessageChannel::Party => format!("[Party] {}:", query.get::<&Account>().expect("Could not find Account").name),
            MessageChannel::Private => {
                if name.is_empty() {
                    return Ok(());
                }

                if name == query.get::<&Account>().expect("Could not find Account").name {
                    return send_fltalert(
                        storage,
                        query.get::<&Socket>().expect("Could not find Socket").id,
                        "You cannot send messages to yourself".into(),
                        FtlType::Error,
                    );
                }

                usersocket = match storage.player_names.borrow().get(&name) {
                    Some(id) => {
                        if let Ok(socket) = world.get::<&Socket>(id.0) {
                            Some(socket.id)
                        } else {
                            return Ok(());
                        }
                    }
                    None => {
                        return send_fltalert(
                            storage,
                            query.get::<&Socket>().expect("Could not find Socket").id,
                            "Player is offline or does not exist".into(),
                            FtlType::Error,
                        );
                    }
                };

                format!("[Private] {}:", query.get::<&Account>().expect("Could not find Account").name)
            }
            MessageChannel::Guild => format!("[Guild] {}:", query.get::<&Account>().expect("Could not find Account").name),
            MessageChannel::Help => format!("[Help] {}:", query.get::<&Account>().expect("Could not find Account").name),
            MessageChannel::Quest => "".into(),
            MessageChannel::Npc => "".into(),
        };

        return send_message(world, storage, entity, msg, head, channel, usersocket);
    }

    Err(AscendingError::InvalidSocket)
}
