use crate::{
    containers::Storage, gameloop::*, gametypes::*, maps::*, players::*, socket::*, sql::*,
};
use bytey::ByteBuffer;
use chrono::Duration;
use phf::phf_map;
use regex::Regex;
use unwrap_helpers::*;

type PacketFunction = fn(&Storage, &mut ByteBuffer, usize) -> Result<()>;

static PACKET_MAP: phf::Map<u32, PacketFunction> = phf_map! {
    0u32 => handle_register,
    1u32 => handle_move,
    2u32 => handle_move,
    3u32 => handle_move,
};

pub fn handle_data(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    let id: u32 = data.read()?;

    if let Some(user) = world.players.borrow().get(uid) {
        match user.borrow().status {
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
    }

    let fun = unwrap_or_return!(PACKET_MAP.get(&id), Err(AscendingError::InvalidPacket));
    fun(world, data, uid)
}

fn handle_register(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    let username = data.read_str()?;
    let password = data.read_str()?;
    let email = data.read_str()?;
    let name = data.read_str()?;
    let sprite: u8 = data.read()?;
    let hair: u8 = data.read()?;

    if let Some(p) = world.players.borrow().get(uid) {
        let mut user = p.borrow_mut();
        let email_regex = Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
        )?;

        if !username.chars().all(is_name_acceptable)
            || !password.chars().all(is_password_acceptable)
            || !name.chars().all(is_name_acceptable)
        {
            return send_infomsg(
                world,
                user.socket_id,
                "Username, Name, or Password contains unaccepted Characters".into(),
                0,
            );
        }

        if username.len() >= 64 || name.len() >= 64 {
            return send_infomsg(
                world,
                user.socket_id,
                "Username or Name has too many Characters, 64 Characters Max".into(),
                0,
            );
        }

        if password.len() >= 128 {
            return send_infomsg(
                world,
                user.socket_id,
                "Password has too many Characters, 128 Characters Max".into(),
                0,
            );
        }

        if !email_regex.is_match(&email) || hair >= 8 || sprite >= 6 {
            return send_infomsg(
                world,
                user.socket_id,
                "Email must be an actual email.".into(),
                0,
            );
        }

        match check_existance(&mut world.pgconn.borrow_mut(), &username, &name, &email) {
            Ok(i) => match i {
                0 => {}
                1 => {
                    return send_infomsg(
                        world,
                        user.socket_id,
                        "Username Exists. Please try Another.".into(),
                        0,
                    )
                }
                2 => {
                    return send_infomsg(
                        world,
                        user.socket_id,
                        "Character Name Exists. Please try Another.".into(),
                        0,
                    )
                }
                3 => {
                    return send_infomsg(
                        world,
                        user.socket_id,
                        "Email Already Exists. Please Try Another.".into(),
                        0,
                    )
                }
                _ => return Err(AscendingError::RegisterFail),
            },
            Err(_) => return Err(AscendingError::UserNotFound),
        }

        user.name = name;
        user.sprite = sprite;

        if new_player(
            &mut world.pgconn.borrow_mut(),
            &user,
            username,
            password,
            email,
        )
        .is_err()
        {
            return send_infomsg(
                world,
                user.socket_id,
                "There was an Issue Creating the player account. Please Contact Support.".into(),
                0,
            );
        }

        return send_infomsg(world, user.socket_id, "Account Was Created. Please wait for the Verification code sent to your email before logging in.".into(), 1);
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_login(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    let username = data.read_str()?;
    let password = data.read_str()?;
    let appmajor = data.read::<u16>()? as usize;
    let appminior = data.read::<u16>()? as usize;
    let apprevision = data.read::<u16>()? as usize;

    if username.len() >= 64 || password.len() >= 128 {
        return send_infomsg(
            world,
            p.borrow().socket_id,
            "Account does not Exist or Password is not Correct.".into(),
            0,
        );
    }

    if let Some(p) = world.players.borrow().get(uid) {
        let id = find_player(&mut world.pgconn.borrow_mut(), &username, &password)?;
        let id = unwrap_or_return!(
            id,
            send_infomsg(
                world,
                p.borrow().socket_id,
                "Account does not Exist or Password is not Correct.".into(),
                1,
            )
        );

        if APP_MAJOR > appmajor && APP_MINOR > appminior && APP_REVISION > apprevision {
            return send_infomsg(
                world,
                p.borrow().socket_id,
                "Client needs to be updated.".into(),
                1,
            );
        }

        p.borrow_mut().accid = id;

        if let Err(_e) = load_player(world, &mut world.pgconn.borrow_mut(), &mut p.borrow_mut()) {
            return send_infomsg(world, p.borrow().socket_id, "Error Loading User.".into(), 1);
        }

        send_loginok(world, p.borrow().socket_id)?;

        //joingame(index);
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_move(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        if !user.borrow().e.life.is_alive()
            || user.borrow().using.inuse()
            || user.borrow().e.stunned
        {
            return Ok(());
        }

        let dir: u8 = data.read()?;
        let pos: Position = data.read()?;

        if world.bases.map.get(&pos.map).is_none() || dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        if pos != user.borrow().e.pos {
            user.borrow_mut().warp(world, user.borrow().e.pos, dir);
            return Ok(());
        }

        user.borrow_mut().movement(world, dir);
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_dir(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        if !user.borrow().e.life.is_alive() || user.borrow().using.inuse() {
            return Ok(());
        }

        let dir = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        user.borrow_mut().e.dir = dir;

        send_dir(world, &user.borrow(), true)?;

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_attack(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        if !user.borrow().e.life.is_alive()
            || user.borrow().using.inuse()
            || user.borrow().e.attacking
        {
            return Ok(());
        }

        let dir = data.read::<u8>()?;
        let _id = data.read::<u8>()?;

        if dir > 3 {
            return Err(AscendingError::InvalidPacket);
        }

        if user.borrow().e.dir != dir {
            user.borrow_mut().e.dir = dir;
            send_dir(world, &user.borrow(), true)?;
        }

        //TODO Add Attack funciton call here for player attacks
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_useitem(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        if !user.borrow().e.life.is_alive()
            || user.borrow().using.inuse()
            || user.borrow().e.attacking
            || user.borrow().e.stunned
            || user.borrow().itemtimer > *world.gettick.borrow()
        {
            return Ok(());
        }

        let _slot = data.read::<u16>()?;
        let _targettype = data.read::<u8>()?;

        user.borrow_mut().itemtimer = *world.gettick.borrow() + Duration::milliseconds(250);
        //TODO useitem();
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_unequip(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        let mut player = user.borrow_mut();
        if !player.e.life.is_alive()
            || player.using.inuse()
            || player.e.attacking
            || player.e.stunned
            || player.itemtimer > *world.gettick.borrow()
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= EQUIPMENT_TYPE_MAX || player.equip[slot].val == 0 {
            return Ok(());
        }

        let mut item = player.equip[slot];
        let rem = give_item(world, &mut player, &mut item);

        if rem > 0 {
            return send_fltalert(
                world,
                player.socket_id,
                "Could not unequiped. No inventory space.".into(),
                FtlType::Error,
            );
        }

        player.equip[slot] = item;
        let _ = update_equipment(&mut world.pgconn.borrow_mut(), &mut player, slot);
        //TODO calculatestats();
        return send_equipment(world, &player);
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_switchinvslot(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        let mut player = user.borrow_mut();
        if !player.e.life.is_alive()
            || player.using.inuse()
            || player.e.attacking
            || player.e.stunned
            || player.itemtimer > *world.gettick.borrow()
        {
            return Ok(());
        }

        let oldslot = data.read::<u16>()? as usize;
        let newslot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if oldslot >= MAX_INV || newslot >= MAX_INV || player.inv[oldslot].val == 0 {
            return Ok(());
        }

        let base1 = &world.bases.item[player.inv[oldslot].num as usize];
        let invtype = get_inv_itemtype(base1);

        if get_inv_type(oldslot) != invtype || get_inv_type(newslot) != invtype {
            return Ok(());
        }

        let mut itemold = player.inv[oldslot];

        if player.inv[newslot].val > 0 {
            if player.inv[newslot].num == player.inv[oldslot].num {
                set_inv_slot(world, &mut player, &mut itemold, newslot, amount);
                player.inv[oldslot] = itemold;
                save_item(world, &mut player, oldslot);
            } else if player.inv[oldslot].val == amount {
                let itemnew = player.inv[newslot];
                player.inv[newslot] = itemold;
                player.inv[oldslot] = itemnew;
                save_item(world, &mut player, newslot);
                save_item(world, &mut player, oldslot);
            } else {
                return send_fltalert(
                        world,
                        player.socket_id,
                        "Can not swap slots with a different containing items unless you swap everything."
                            .into(),
                        FtlType::Error
                    );
            }
        } else {
            set_inv_slot(world, &mut player, &mut itemold, newslot, amount);
            player.inv[oldslot] = itemold;
            save_item(world, &mut player, oldslot);
        }

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_pickup(world: &Storage, _data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        let mut player = user.borrow_mut();
        let mut remid: Option<(MapPosition, usize)> = None;

        if !player.e.life.is_alive()
            || player.using.inuse()
            || player.e.attacking
            || player.e.stunned
            || player.mapitemtimer > *world.gettick.borrow()
        {
            return Ok(());
        }

        let mapids = get_maps_in_range(world, &player.e.pos, 1);

        'remremove: for id in mapids {
            if let Some(x) = id.get() {
                let mut map = unwrap_continue!(world.map_data.get(&x)).borrow_mut();
                let _ = unwrap_continue!(world.bases.map.get(&player.e.pos.map));
                let ids = map.itemids.clone();

                for i in ids {
                    if player
                        .e
                        .pos
                        .checkdistance(map.items[i].pos.map_offset(id.into()))
                        <= 1
                    {
                        if map.items[i].item.num == 0 {
                            let rem = give_vals(world, &mut player, map.items[i].item.val as u64);
                            map.items[i].item.val = rem as u16;

                            if rem == 0 {
                                remid = Some((x, i));
                                break 'remremove;
                            }
                        } else {
                            let amount = map.items[i].item.val;
                            let rem = give_item(world, &mut player, &mut map.items[i].item);
                            let item = &world.bases.item[map.items[i].item.num as usize];

                            if rem == 0 {
                                let st = match amount {
                                    0 | 1 => "",
                                    _ => "'s",
                                };

                                let _ = send_fltalert(
                                    world,
                                    player.socket_id,
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
                                        world,
                                        player.socket_id,
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
            if let Some(map) = world.map_data.get(&id.0) {
                map.borrow_mut().remove_item(id.1);
                let _ = send_data_remove(world, id.1 as u64, id.0, 3);
            }
        }

        player.mapitemtimer = *world.gettick.borrow() + Duration::milliseconds(100);
        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_dropitem(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        let mut player = user.borrow_mut();
        if !player.e.life.is_alive()
            || player.using.inuse()
            || player.e.attacking
            || player.e.stunned
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;
        let amount = data.read::<u16>()?;

        if slot >= MAX_INV || player.inv[slot].val == 0 || amount == 0 {
            return Ok(());
        }

        //make sure it exists first.
        let _ = unwrap_or_return!(
            world.bases.map.get(&player.e.pos.map),
            Err(AscendingError::Unhandled)
        );

        match get_inv_type(slot) {
            InvType::Quest | InvType::Key => {
                return send_fltalert(
                    world,
                    player.socket_id,
                    "You can not drop key or Quest items.".into(),
                    FtlType::Error,
                );
            }
            _ => {}
        }

        let mut mapitem = MapItem::new(0);

        mapitem.item = player.inv[slot];
        mapitem.despawn = match player.access {
            UserAccess::Admin => None,
            _ => Some(*world.gettick.borrow() + Duration::milliseconds(600000)),
        };
        mapitem.ownertimer = Some(*world.gettick.borrow() + Duration::milliseconds(5000));
        mapitem.ownerid = player.accid;
        mapitem.pos = player.e.pos;

        let leftover = take_itemslot(world, &mut player, slot, amount);
        mapitem.item.val -= leftover;
        let mut map = unwrap_or_return!(
            world.map_data.get(&player.e.pos.map),
            Err(AscendingError::Unhandled)
        )
        .borrow_mut();

        map.add_mapitem(mapitem);

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_deleteitem(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        let mut player = user.borrow_mut();
        if !player.e.life.is_alive()
            || player.using.inuse()
            || player.e.attacking
            || player.e.stunned
        {
            return Ok(());
        }

        let slot = data.read::<u16>()? as usize;

        if slot >= MAX_INV || player.inv[slot].val == 0 {
            return Ok(());
        }

        match get_inv_type(slot) {
            InvType::Quest | InvType::Key => {
                return send_fltalert(
                    world,
                    player.socket_id,
                    "You can not delete key or Quest items.".into(),
                    FtlType::Error,
                );
            }
            _ => {}
        }
        let val = player.inv[slot].val;
        let _ = take_itemslot(world, &mut player, slot, val);

        return Ok(());
    }

    Err(AscendingError::InvalidSocket)
}

fn handle_message(world: &Storage, data: &mut ByteBuffer, uid: usize) -> Result<()> {
    if let Some(user) = world.players.borrow().get(uid) {
        let player = user.borrow();
        let mut usersocket: Option<usize> = None;

        if !player.e.life.is_alive()
            || player.using.inuse()
            || player.e.attacking
            || player.e.stunned
        {
            return Ok(());
        }

        let channel: MessageChannel = data.read()?;

        let msg = data.read_str()?;
        let name = data.read_str()?;

        if msg.len() >= 256 {
            return send_fltalert(
                world,
                player.socket_id,
                "Your message is too long. (256 character limit)".into(),
                FtlType::Error,
            );
        }

        let head = match channel {
            MessageChannel::Map => format!("[Map] {}:", player.name),
            MessageChannel::Global => format!("[Global] {}:", player.name),
            MessageChannel::Trade => format!("[Trade] {}:", player.name),
            MessageChannel::Party => format!("[Party] {}:", player.name),
            MessageChannel::Private => {
                if name.is_empty() {
                    return Ok(());
                }

                if name == player.name {
                    return send_fltalert(
                        world,
                        player.socket_id,
                        "You cannot send messages to yourself".into(),
                        FtlType::Error,
                    );
                }

                usersocket = match world.name_map.borrow().get(&name) {
                    Some(id) => {
                        if let Some(user2) = world.players.borrow().get(*id) {
                            Some(user2.borrow().socket_id)
                        } else {
                            return Ok(());
                        }
                    }
                    None => {
                        return send_fltalert(
                            world,
                            player.socket_id,
                            "Player is offline or does not exist".into(),
                            FtlType::Error,
                        );
                    }
                };

                format!("[Private] {}:", player.name)
            }
            MessageChannel::Guild => format!("[Guild] {}:", player.name),
            MessageChannel::Help => format!("[Help] {}:", player.name),
            MessageChannel::Quest => "".into(),
            MessageChannel::Npc => "".into(),
        };

        return send_message(world, &player, msg, head, channel, usersocket);
    }

    Err(AscendingError::InvalidSocket)
}
