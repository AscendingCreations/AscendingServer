use crate::{
    containers::Storage, gameloop::*, gametypes::*, maps::*, players::*, sql::*, tasks::*,
};
use bytey::ByteBuffer;
use chrono::Duration;
use hecs::World;
use regex::Regex;

pub fn handle_register(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let email = data.read::<String>()?;
    let sprite: u8 = data.read()?;

    let socket_id = world.get::<&Socket>(entity.0).unwrap().id;

    if !storage.player_ids.borrow().contains(entity) {
        let email_regex = Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )?;

        if !username.chars().all(is_name_acceptable)
            || !password.chars().all(is_password_acceptable)
        {
            return send_infomsg(
                storage,
                socket_id,
                "Username or Password contains unaccepted Characters".into(),
                0,
            );
        }

        if username.len() >= 64 {
            return send_infomsg(
                storage,
                socket_id,
                "Username has too many Characters, 64 Characters Max".into(),
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

        if !email_regex.is_match(&email) || sprite >= 6 {
            return send_infomsg(
                storage,
                socket_id,
                "Email must be an actual email.".into(),
                0,
            );
        }

        match check_existance(storage, &username, &email) {
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
                        "Email Already Exists. Please Try Another.".into(),
                        0,
                    )
                }
                _ => return Err(AscendingError::RegisterFail),
            },
            Err(_) => return Err(AscendingError::UserNotFound),
        }

        // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
        storage.add_player_data(world, entity);

        {
            world
                .get::<&mut Account>(entity.0)
                .expect("Could not find Account")
                .username
                .clone_from(&username);
            world
                .get::<&mut Sprite>(entity.0)
                .expect("Could not find Sprite")
                .id = sprite as u16;
        }

        let res = new_player(storage, world, entity, username, email, password);
        if let Err(e) = res {
            println!("{}", e);
            return send_infomsg(
                storage,
                socket_id,
                "There was an Issue Creating the player account. Please Contact Support.".into(),
                0,
            );
        }

        joingame(world, storage, entity);
        return Ok(());
        //return send_infomsg(storage, socket_id,
        //     "Account Was Created. Please wait for the Verification code sent to your email before logging in.".into(), 1);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_login(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    let username = data.read::<String>()?;
    let password = data.read::<String>()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;

    let socket_id = world.get::<&Socket>(entity.0).unwrap().id;

    if !storage.player_ids.borrow().contains(entity) {
        if username.len() >= 64 || password.len() >= 128 {
            return send_infomsg(
                storage,
                socket_id,
                "Account does not Exist or Password is not Correct.".into(),
                0,
            );
        }

        let id = match find_player(storage, &username, &password)? {
            Some(id) => id,
            None => {
                return send_infomsg(
                    storage,
                    socket_id,
                    "Account does not Exist or Password is not Correct.".into(),
                    1,
                )
            }
        };

        if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
            return send_infomsg(storage, socket_id, "Client needs to be updated.".into(), 1);
        }

        // we need to Add all the player types creations in a sub function that Creates the Defaults and then adds them to World.
        storage.add_player_data(world, entity);

        world
            .get::<&mut Account>(entity.0)
            .expect("Could not find Account")
            .id = id;

        if let Err(_e) = load_player(storage, world, entity) {
            return send_infomsg(storage, socket_id, "Error Loading User.".into(), 1);
        }

        joingame(world, storage, entity);
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_move(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Stunned>(p).0
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let data_pos = data.read::<Position>()?;

        if storage.bases.maps.get(&data_pos.map).is_none() || dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        let pos = world.get_or_panic::<Position>(p);

        if data_pos != pos {
            player_warp(world, storage, entity, &data_pos, false);
            return Ok(());
        }

        player_movement(world, storage, entity, dir);
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_dir(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        {
            world.get::<&mut Dir>(p.0).expect("Could not find Dir").0 = dir;
        }

        send_dir(world, storage, entity, true)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_attack(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<AttackTimer>(p).0 > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let target = data.read::<Option<Entity>>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        if world.get_or_panic::<Dir>(p).0 != dir {
            {
                world
                    .get::<&mut Dir>(entity.0)
                    .expect("Could not find Dir")
                    .0 = dir;
            }
            send_dir(world, storage, entity, true)?;
        };

        if let Some(target_entity) = target {
            if world.contains(target_entity.0) {
                player_combat(world, storage, entity, &target_entity);
                {
                    world
                        .get::<&mut AttackTimer>(entity.0)
                        .expect("Could not find AttackTimer")
                        .0 =
                        *storage.gettick.borrow() + Duration::try_milliseconds(250).unwrap_or_default();
                }
            }
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_useitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<Stunned>(p).0
            || world.get_or_panic::<PlayerItemTimer>(p).itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let _slot = data.read::<u16>()?;
        let _targettype = data.read::<u8>()?;

        {
            world
                .get::<&mut PlayerItemTimer>(entity.0)
                .expect("Could not find PlayerItemTimer")
                .itemtimer =
                *storage.gettick.borrow() + Duration::try_milliseconds(250).unwrap_or_default();
        }

        //TODO useitem();
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_unequip(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<Stunned>(p).0
            || world.get_or_panic::<PlayerItemTimer>(p).itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= EQUIPMENT_TYPE_MAX || world.get::<&Equipment>(p.0).unwrap().items[slot].val == 0
        {
            return Ok(());
        }

        let mut item = world.get::<&Equipment>(p.0).unwrap().items[slot];
        let rem = give_item(world, storage, p, &mut item);

        if rem > 0 {
            return send_fltalert(
                storage,
                world.get::<&Socket>(p.0).unwrap().id,
                "Could not unequiped. No inventory space.".into(),
                FtlType::Error,
            );
        }

        {
            world
                .get::<&mut Equipment>(p.0)
                .expect("Could not find Equipment")
                .items[slot] = item;
        }

        let _ = update_equipment(storage, world, p, slot);
        //TODO calculatestats();
        return send_equipment(world, storage, p);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_switchinvslot(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<Stunned>(p).0
            || world.get_or_panic::<PlayerItemTimer>(p).itemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if oldslot >= MAX_INV
            || newslot >= MAX_INV
            || world.get::<&Inventory>(p.0).unwrap().items[oldslot].val == 0
        {
            return Ok(());
        }

        let base1 =
            &storage.bases.items[world.get::<&Inventory>(p.0).unwrap().items[oldslot].num as usize];
        let invtype = get_inv_itemtype(base1);

        if get_inv_type(oldslot) != invtype || get_inv_type(newslot) != invtype {
            return Ok(());
        }

        let mut itemold = world.get::<&Inventory>(p.0).unwrap().items[oldslot];

        if world.get::<&Inventory>(p.0).unwrap().items[newslot].val > 0 {
            if world.get::<&Inventory>(p.0).unwrap().items[newslot].num
                == world.get::<&Inventory>(p.0).unwrap().items[oldslot].num
            {
                set_inv_slot(world, storage, entity, &mut itemold, newslot, amount);
                {
                    world
                        .get::<&mut Inventory>(p.0)
                        .expect("Could not find Inventory")
                        .items[oldslot] = itemold;
                }
                save_item(world, storage, entity, oldslot);
            } else if world.get::<&Inventory>(p.0).unwrap().items[oldslot].val == amount {
                let itemnew = world.get::<&Inventory>(p.0).unwrap().items[newslot];
                {
                    world
                        .get::<&mut Inventory>(p.0)
                        .expect("Could not find Inventory")
                        .items[newslot] = itemold;
                    world
                        .get::<&mut Inventory>(p.0)
                        .expect("Could not find Inventory")
                        .items[oldslot] = itemnew;
                }
                save_item(world, storage, entity, newslot);
                save_item(world, storage, entity, oldslot);
            } else {
                return send_fltalert(
                        storage,
                        world.get::<&Socket>(p.0).unwrap().id,
                        "Can not swap slots with a different containing items unless you swap everything."
                            .into(),
                        FtlType::Error
                    );
            }
        } else {
            set_inv_slot(world, storage, entity, &mut itemold, newslot, amount);
            {
                world
                    .get::<&mut Inventory>(p.0)
                    .expect("Could not find Inventory")
                    .items[oldslot] = itemold;
            }
            save_item(world, storage, entity, oldslot);
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_pickup(
    world: &mut World,
    storage: &Storage,
    _data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        let mut remid: Option<(MapPosition, Entity)> = None;

        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<Stunned>(p).0
            || world.get_or_panic::<PlayerMapTimer>(p).mapitemtimer > *storage.gettick.borrow()
        {
            return Ok(());
        }

        let mapids = get_maps_in_range(storage, &world.get_or_panic::<Position>(p), 1);

        'remremove: for id in mapids {
            if let Some(x) = id.get() {
                let map = match storage.maps.get(&x) {
                    Some(map) => map,
                    None => continue,
                }
                .borrow_mut();

                // for the map base data when we need it.
                if storage
                    .bases
                    .maps
                    .get(&world.get_or_panic::<Position>(p).map)
                    .is_none()
                {
                    continue;
                }
                let ids = map.itemids.clone();

                for i in ids {
                    let mut mapitems = world.cloned_get_or_panic::<MapItem>(p);
                    if world
                        .get_or_panic::<Position>(p)
                        .checkdistance(mapitems.pos.map_offset(id.into()))
                        <= 1
                    {
                        if mapitems.item.num == 0 {
                            let rem =
                                player_give_vals(world, storage, entity, mapitems.item.val as u64);
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
                                    world.get::<&Socket>(p.0).unwrap().id,
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
                                        world.get::<&Socket>(p.0).unwrap().id,
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
                let _ = DataTaskToken::EntityUnload(id.0).add_task(storage, &(id.1));
            }
        }
        {
            world
                .get::<&mut PlayerMapTimer>(p.0)
                .expect("Could not find PlayerMapTimer")
                .mapitemtimer =
                *storage.gettick.borrow() + Duration::try_milliseconds(100).unwrap_or_default();
        }
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_dropitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<Stunned>(p).0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if slot >= MAX_INV
            || world.get::<&Inventory>(p.0).unwrap().items[slot].val == 0
            || amount == 0
        {
            return Ok(());
        }

        //make sure it exists first.
        if !storage
            .bases
            .maps
            .contains_key(&world.get_or_panic::<Position>(p).map)
        {
            return Err(AscendingError::Unhandled);
        }

        match get_inv_type(slot) {
            InvType::Quest | InvType::Key => {
                return send_fltalert(
                    storage,
                    world.get::<&Socket>(p.0).unwrap().id,
                    "You can not drop key or Quest items.".into(),
                    FtlType::Error,
                );
            }
            _ => {}
        }

        let mut mapitem = MapItem::new(0);

        mapitem.item = world.get::<&Inventory>(p.0).unwrap().items[slot];
        mapitem.despawn = match world.get_or_panic::<UserAccess>(p) {
            UserAccess::Admin => None,
            _ => Some(
                *storage.gettick.borrow() + Duration::try_milliseconds(600000).unwrap_or_default(),
            ),
        };
        mapitem.ownertimer =
            Some(*storage.gettick.borrow() + Duration::try_milliseconds(5000).unwrap_or_default());
        mapitem.ownerid = Some(*p);
        mapitem.pos = world.get_or_panic::<Position>(p);

        let leftover = take_itemslot(world, storage, entity, slot, amount);
        mapitem.item.val -= leftover;
        let mut map = match storage.maps.get(&world.get_or_panic::<Position>(p).map) {
            Some(map) => map,
            None => return Err(AscendingError::Unhandled),
        }
        .borrow_mut();

        let id = map.add_mapitem(world, mapitem);
        let _ = DataTaskToken::ItemLoad(world.get_or_panic::<Position>(p).map).add_task(
            storage,
            &MapItemPacket::new(id, mapitem.pos, mapitem.item, mapitem.ownerid),
        );

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_deleteitem(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<Stunned>(p).0
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= MAX_INV || world.get::<&Inventory>(p.0).unwrap().items[slot].val == 0 {
            return Ok(());
        }

        match get_inv_type(slot) {
            InvType::Quest | InvType::Key => {
                return send_fltalert(
                    storage,
                    world.get::<&Socket>(entity.0).unwrap().id,
                    "You can not delete key or Quest items.".into(),
                    FtlType::Error,
                );
            }
            _ => {}
        }
        let val = world.get::<&Inventory>(p.0).unwrap().items[slot].val;
        let _ = take_itemslot(world, storage, entity, slot, val);

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_message(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(p) = storage.player_ids.borrow().get(entity) {
        let mut usersocket: Option<usize> = None;

        if !world.get_or_panic::<DeathType>(p).is_alive()
            || world.get_or_panic::<IsUsingType>(p).inuse()
            || world.get_or_panic::<Attacking>(p).0
            || world.get_or_panic::<Stunned>(p).0
        {
            return Ok(());
        }

        let channel: MessageChannel = data.read()?;

        let msg = data.read::<String>()?;
        let name = data.read::<String>()?;

        if msg.len() >= 256 {
            return send_fltalert(
                storage,
                world.get::<&Socket>(entity.0).unwrap().id,
                "Your message is too long. (256 character limit)".into(),
                FtlType::Error,
            );
        }

        let head = match channel {
            MessageChannel::Map => {
                format!("[Map] {}:", world.get::<&Account>(p.0).unwrap().username)
            }
            MessageChannel::Global => {
                format!("[Global] {}:", world.get::<&Account>(p.0).unwrap().username)
            }
            MessageChannel::Trade => {
                format!("[Trade] {}:", world.get::<&Account>(p.0).unwrap().username)
            }
            MessageChannel::Party => {
                format!("[Party] {}:", world.get::<&Account>(p.0).unwrap().username)
            }
            MessageChannel::Private => {
                if name.is_empty() {
                    return Ok(());
                }

                if name == world.get::<&Account>(p.0).unwrap().username {
                    return send_fltalert(
                        storage,
                        world.get::<&Socket>(entity.0).unwrap().id,
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
                            world.get::<&Socket>(entity.0).unwrap().id,
                            "Player is offline or does not exist".into(),
                            FtlType::Error,
                        );
                    }
                };

                format!(
                    "[Private] {}:",
                    world.get::<&Account>(p.0).unwrap().username
                )
            }
            MessageChannel::Guild => {
                format!("[Guild] {}:", world.get::<&Account>(p.0).unwrap().username)
            }
            MessageChannel::Help => {
                format!("[Help] {}:", world.get::<&Account>(p.0).unwrap().username)
            }
            MessageChannel::Quest => "".into(),
            MessageChannel::Npc => "".into(),
        };

        return send_message(world, storage, entity, msg, head, channel, usersocket);
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_admincommand(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(_p) = storage.player_ids.borrow().get(entity) {
        let command = data.read::<AdminCommand>()?;

        match command {
            AdminCommand::KickPlayer(name) => {
                println!("Kicking Player {:?}", name);
            }
            AdminCommand::WarpTo(pos) => {
                println!("Warping to {:?}", pos);
            }
            AdminCommand::SpawnNpc(index, pos) => {
                println!("Spawning NPC {index} on {:?}", pos);
                if let Some(mapdata) = storage.maps.get(&pos.map) {
                    let mut data = mapdata.borrow_mut();
                    if let Ok(id) = storage.add_npc(world, index as u64) {
                        data.add_npc(id);
                        spawn_npc(world, pos, None, id);
                    }
                }
            }
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

pub fn handle_settarget(
    world: &mut World,
    storage: &Storage,
    data: &mut ByteBuffer,
    entity: &Entity,
) -> Result<()> {
    if let Some(_p) = storage.player_ids.borrow().get(entity) {
        let target = data.read::<Option<Entity>>()?;

        if let Some(target_entity) = target {
            if !world.contains(target_entity.0) {
                return Ok(());
            }
        }
        world.get::<&mut PlayerTarget>(entity.0).expect("Could not find PlayerTarget").0 =
            target;
        
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}