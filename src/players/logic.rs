use crate::{
    containers::{
        DeathType, Entity, GlobalKey, IsUsingType, Storage, Target, TradeItem, TradeMoney,
        TradeRequestEntity, TradeStatus, World,
    },
    gametypes::*,
    maps::can_target,
    npcs::npc_clear_move_path,
    players::*,
    socket::*,
    sql::*,
    tasks::*,
};
use chrono::Duration;
use log::debug;
use std::cmp;

pub fn update_players(world: &mut World, storage: &Storage) -> Result<()> {
    let tick = *storage.gettick.borrow();

    for id in &*storage.player_ids.borrow() {
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(*id) {
            let (spawn, onlinetype, deathtype, deathtimer, pos, is_using_type) = {
                let p_data = p_data.try_lock()?;

                (
                    p_data.movement.spawn,
                    p_data.online_type,
                    p_data.combat.death_type,
                    p_data.combat.death_timer,
                    p_data.movement.pos,
                    p_data.is_using_type,
                )
            };

            if onlinetype == OnlineType::Online {
                if deathtype == DeathType::Spirit {
                    //timers
                    if deathtimer.0 < tick {
                        {
                            let mut p_data = p_data.try_lock()?;

                            p_data.combat.death_type = DeathType::Alive;

                            //lets heal them fully on revival.
                            for i in 0..VITALS_MAX {
                                p_data.combat.vitals.vital[i] = p_data.combat.vitals.vitalmax[i];
                            }
                        }

                        player_warp(world, storage, *id, &spawn.pos, false)?;

                        DataTaskToken::Death(pos.map)
                            .add_task(storage, death_packet(*id, DeathType::Alive)?)?;
                    }
                }

                // Check Trade
                if let IsUsingType::Trading(tradeentity) = is_using_type {
                    if !world.entities.contains_key(tradeentity) {
                        close_trade(world, storage, *id)?;
                        send_message(
                            world,
                            storage,
                            *id,
                            "Trade has cancelled the trade".into(),
                            String::new(),
                            MessageChannel::Private,
                            None,
                        )?;

                        p_data.try_lock()?.trade_request_entity = TradeRequestEntity::default();
                    }
                };
            }
        }
    }

    Ok(())
}

pub fn check_player_connection(world: &mut World, storage: &Storage) -> Result<()> {
    let mut remove_player_list = Vec::new();

    let tick = *storage.gettick.borrow();

    for (entity, connection_timer) in storage.player_timeout.borrow().iter() {
        if connection_timer.0 < tick {
            remove_player_list.push(entity);
        }
    }

    for i in remove_player_list.iter() {
        // Close Socket
        disconnect(*i, world, storage)?;
    }
    Ok(())
}

//If they login successfully the remove the timer from world.
pub fn send_connection_pings(world: &mut World, storage: &Storage) -> Result<()> {
    for id in &*storage.player_ids.borrow() {
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(*id) {
            let online_type = { p_data.try_lock()?.online_type };

            if online_type == OnlineType::Online {
                send_ping(world, storage, *id)?;
            }
        }
    }

    Ok(())
}

pub fn player_earn_exp(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    victimlevel: i32,
    expval: i64,
    spercent: f64,
) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut giveexp = expval;

        giveexp = (giveexp as f64 * spercent) as i64;

        let (mut cur_level, socket_id, position) = {
            let mut p_data = p_data.try_lock()?;

            if p_data.combat.level >= MAX_LVL as i32 || expval == 0 {
                return Ok(());
            }

            p_data.combat.in_combat = true;
            p_data.combat.combat_timer.0 =
                *storage.gettick.borrow() + Duration::try_milliseconds(2000).unwrap_or_default();

            (p_data.combat.level, p_data.socket.id, p_data.movement.pos)
        };

        let leveldifference = victimlevel - cur_level;

        if (1..=5).contains(&leveldifference) {
            giveexp = (giveexp as f64 * 1.1) as i64;
        } else if leveldifference <= -1 {
            giveexp += (giveexp as f64 * (leveldifference as f64 * 0.1)) as i64;
        }

        let mut levelexp = {
            let mut p_data = p_data.try_lock()?;

            p_data.general.levelexp += cmp::max(giveexp, 1) as u64;

            p_data.general.levelexp
        };

        while levelexp >= player_get_next_lvl_exp(world, entity)? && cur_level != MAX_LVL as i32 {
            {
                let mut p_data = p_data.try_lock()?;

                p_data.combat.level += 1;
                p_data.general.levelexp = p_data
                    .general
                    .levelexp
                    .saturating_sub(player_get_next_lvl_exp(world, entity)?);

                cur_level = p_data.combat.level;
                levelexp = p_data.general.levelexp;
            }

            let maxhp = player_calc_max_hp(world, entity)?;
            let maxmp = player_calc_max_mp(world, entity)?;

            {
                let mut p_data = p_data.try_lock()?;

                p_data.combat.vitals.vitalmax[VitalTypes::Hp as usize] = maxhp;
                p_data.combat.vitals.vitalmax[VitalTypes::Mp as usize] = maxmp;
            }

            send_message(
                world,
                storage,
                entity,
                "You have gained a level!".into(),
                String::new(),
                MessageChannel::Private,
                None,
            )?;

            send_fltalert(storage, socket_id, "Level Up!.".into(), FtlType::Level)?;
        }

        for i in 0..VitalTypes::Count as usize {
            p_data.try_lock()?.combat.vitals.vital[i] = player_add_up_vital(world, entity, i)?;
        }

        let vitals = { p_data.try_lock()?.combat.vitals };

        send_level(world, storage, entity)?;
        DataTaskToken::Vitals(position.map).add_task(
            storage,
            vitals_packet(entity, vitals.vital, vitals.vitalmax)?,
        )?;
        update_level(storage, world, entity)?;
    }
    Ok(())
}

pub fn player_get_next_lvl_exp(world: &mut World, entity: GlobalKey) -> Result<u64> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let level = p_data.try_lock()?.combat.level;

        let exp_per_level = match level {
            1..=10 => 100,
            11..=20 => 250,
            21..=30 => 400,
            31..=40 => 550,
            41..=50 => 700,
            51..=60 => 850,
            61..=70 => 1000,
            71..=80 => 1150,
            81..=90 => 1300,
            91..=100 => 1450,
            101..=120 => 2000,
            121..=150 => 3000,
            151..=199 => 4000,
            _ => 0,
        };

        Ok(level as u64 * exp_per_level as u64)
    } else {
        Ok(0)
    }
}

pub fn player_calc_max_hp(world: &mut World, entity: GlobalKey) -> Result<i32> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        Ok(p_data.try_lock()?.combat.level * 25)
    } else {
        Ok(0)
    }
}

pub fn player_calc_max_mp(world: &mut World, entity: GlobalKey) -> Result<i32> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        Ok(p_data.try_lock()?.combat.level * 25)
    } else {
        Ok(0)
    }
}

pub fn player_get_weapon_damage(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
) -> Result<(i16, i16)> {
    Ok(
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
            let p_data = p_data.try_lock()?;

            let mut dmg = (0, 0);

            if p_data.equipment.items[EquipmentType::Weapon as usize].val > 0 {
                if let Some(item) = storage
                    .bases
                    .items
                    .get(p_data.equipment.items[EquipmentType::Weapon as usize].num as usize)
                {
                    dmg = (item.data[0], item.data[1]);
                }
            }

            dmg
        } else {
            (0, 0)
        },
    )
}

pub fn player_get_armor_defense(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
) -> Result<(i16, i16)> {
    Ok(
        if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
            let p_data = p_data.try_lock()?;

            let mut defense = (0i16, 0i16);

            for i in EquipmentType::Helmet as usize..=EquipmentType::Accessory as usize {
                if let Some(item) = storage
                    .bases
                    .items
                    .get(p_data.equipment.items[i].num as usize)
                {
                    defense.0 = defense.0.saturating_add(item.data[0]);
                    defense.1 = defense.1.saturating_add(item.data[1]);
                }
            }

            defense
        } else {
            (0, 0)
        },
    )
}

pub fn player_repair_equipment(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    slot: usize,
    repair_per: f32,
) -> Result<()> {
    let mut update = false;

    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        if let Some(item) = storage
            .bases
            .items
            .get(p_data.equipment.items[slot].num as usize)
        {
            if !item.repairable
                || p_data.equipment.items[slot].data[0] == p_data.equipment.items[slot].data[1]
            {
                return Ok(());
            }

            let repair_amount = (p_data.equipment.items[slot].data[0] as f32 * repair_per) as i16;
            let repair_amount = cmp::min(
                repair_amount,
                p_data.equipment.items[slot].data[0] - p_data.equipment.items[slot].data[1],
            );

            {
                p_data.equipment.items[slot].data[0] =
                    p_data.equipment.items[slot].data[0].saturating_add(repair_amount);
            }

            update = true;
        }
    }

    if update {
        send_equipment(world, storage, entity)?;
        update_equipment(storage, world, entity, slot)?;
    }

    Ok(())
}

pub fn get_next_stat_exp(level: u32) -> u64 {
    let exp_per_level = match level {
        1..=10 => 100,
        11..=20 => 250,
        21..=30 => 400,
        31..=40 => 550,
        41..=50 => 700,
        51..=60 => 850,
        61..=70 => 1000,
        71..=80 => 1150,
        81..=90 => 1300,
        91..=100 => 1450,
        _ => 0,
    };

    level as u64 * exp_per_level as u64
}

pub fn joingame(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let (socket_id, account, position) = {
            let mut p_data = p_data.try_lock()?;

            p_data.online_type = OnlineType::Online;

            (
                p_data.socket.id,
                p_data.account.clone(),
                p_data.movement.pos,
            )
        };

        // Send player index and data
        //send_myindex(storage, socket_id, entity)?;
        send_playerdata(world, storage, socket_id, entity)?;

        // Set player position based on the loaded data
        player_warp(world, storage, entity, &position, true)?;

        // Add player on map
        if let Some(mapref) = storage.maps.get(&position.map) {
            let mut map = mapref.borrow_mut();
            map.add_player(storage, entity);
            map.add_entity_to_grid(position);
        }

        send_inv(world, storage, entity)?;
        send_level(world, storage, entity)?;
        send_money(world, storage, entity)?;

        DataTaskToken::MapChat(position.map).add_task(
            storage,
            message_packet(
                MessageChannel::Map,
                String::new(),
                format!("{} has joined the game", account.username),
                None,
            )?,
        )?;

        debug!("Login Ok");
        // Finish loading
        send_loginok(storage, socket_id)?;
        send_message(
            world,
            storage,
            entity,
            "Welcome Message".into(),
            String::new(),
            MessageChannel::Private,
            None,
        )?;
    }
    Ok(())
}

pub fn left_game(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let (online_type, position, account) = {
            let p_data = p_data.try_lock()?;

            (
                p_data.online_type,
                p_data.movement.pos,
                p_data.account.clone(),
            )
        };

        if online_type != OnlineType::Online {
            return Ok(());
        }

        DataTaskToken::MapChat(position.map).add_task(
            storage,
            message_packet(
                MessageChannel::Map,
                String::new(),
                format!("{} has left the game", account.username),
                None,
            )?,
        )?;

        update_player(storage, world, entity)?;
        update_level(storage, world, entity)?;
        update_pos(storage, world, entity)?;
        update_currency(storage, world, entity)?;
        update_resetcount(storage, world, entity)?;

        //todo Add Update Players on map here.
    }

    Ok(())
}

pub fn remove_all_npc_target(world: &mut World, entity: GlobalKey) -> Result<()> {
    let entities: Vec<_> = world
        .entities
        .iter()
        .filter(|(_key, data)| {
            let mut result = false;

            let data_entity = *data;

            if let Entity::Npc(data) = data_entity.clone() {
                if let Ok(data) = data.try_lock() {
                    if let Some(t_entity) = data.combat.target.target_entity {
                        result = t_entity == entity
                    }
                }
            }

            result
        })
        .map(|(key, _)| key)
        .collect();

    for n_entity in entities {
        if let Some(Entity::Npc(data)) = world.get_opt_entity(n_entity) {
            {
                data.try_lock()?.combat.target = Target::default();
            }
        }

        npc_clear_move_path(world, entity)?;
    }

    Ok(())
}

pub fn init_trade(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<()> {
    if entity == target_entity {
        return Ok(());
    }

    if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
        if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
            let (p1_position, p2_position, p2_death_type, p1_isusingtype, p2_isusingtype) = {
                let p1_data = p1_data.try_lock()?;
                let p2_data = p2_data.try_lock()?;

                (
                    p1_data.movement.pos,
                    p2_data.movement.pos,
                    p2_data.combat.death_type,
                    p1_data.is_using_type,
                    p2_data.is_using_type,
                )
            };

            if p1_isusingtype.inuse() || p2_isusingtype.inuse() {
                return send_message(
                    world,
                    storage,
                    entity,
                    "Player is busy".to_string(),
                    String::new(),
                    MessageChannel::Private,
                    None,
                );
            }

            if !can_target(p1_position, p2_position, p2_death_type, 2) {
                return send_message(
                    world,
                    storage,
                    entity,
                    "Player is not in range".to_string(),
                    String::new(),
                    MessageChannel::Private,
                    None,
                );
            }

            {
                let mut p1_data = p1_data.try_lock()?;
                let mut p2_data = p2_data.try_lock()?;

                p1_data.is_using_type = IsUsingType::Trading(target_entity);
                p2_data.is_using_type = IsUsingType::Trading(entity);

                p1_data.trade_item = TradeItem::default();
                p2_data.trade_item = TradeItem::default();

                p1_data.trade_money = TradeMoney::default();
                p2_data.trade_money = TradeMoney::default();

                p1_data.trade_status = TradeStatus::default();
                p2_data.trade_status = TradeStatus::default();
            }

            send_inittrade(world, storage, entity, target_entity)?;
            send_inittrade(world, storage, target_entity, entity)?;
        }
    }
    Ok(())
}

pub fn process_player_trade(
    world: &mut World,
    storage: &Storage,
    entity: GlobalKey,
    target_entity: GlobalKey,
) -> Result<bool> {
    if entity == target_entity {
        return Ok(false);
    }

    if let Some(Entity::Player(p1_data)) = world.get_opt_entity(entity) {
        if let Some(Entity::Player(p2_data)) = world.get_opt_entity(target_entity) {
            let (
                entity_item,
                target_item,
                entity_money,
                target_money,
                mut entity_clone_inv,
                mut target_clone_inv,
            ) = {
                let p1_data = p1_data.try_lock()?;
                let p2_data = p2_data.try_lock()?;

                (
                    p1_data.trade_item.clone(),
                    p2_data.trade_item.clone(),
                    p1_data.trade_money.vals,
                    p2_data.trade_money.vals,
                    p1_data.inventory.clone(),
                    p2_data.inventory.clone(),
                )
            };

            for item in entity_item.items.clone().iter_mut() {
                if item.val > 0 && !check_temp_inv_space(storage, item, &mut target_clone_inv)? {
                    return Ok(false);
                }
            }
            for item in target_item.items.clone().iter_mut() {
                if item.val > 0 && !check_temp_inv_space(storage, item, &mut entity_clone_inv)? {
                    return Ok(false);
                }
            }

            for item in entity_item.items.iter() {
                if item.val > 0 {
                    take_inv_items(world, storage, entity, item.num, item.val)?;
                }
            }
            player_take_vals(world, storage, entity, entity_money)?;
            for item in target_item.items.iter() {
                if item.val > 0 {
                    take_inv_items(world, storage, target_entity, item.num, item.val)?;
                }
            }
            player_take_vals(world, storage, target_entity, target_money)?;

            for item in entity_item.items.clone().iter_mut() {
                if item.val > 0 {
                    give_inv_item(world, storage, target_entity, item)?;
                }
            }
            player_give_vals(world, storage, target_entity, entity_money)?;
            for item in target_item.items.clone().iter_mut() {
                if item.val > 0 {
                    give_inv_item(world, storage, entity, item)?;
                }
            }
            player_give_vals(world, storage, entity, target_money)?;

            return Ok(true);
        }
    }
    Ok(false)
}

pub fn close_trade(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<()> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let mut p_data = p_data.try_lock()?;

        p_data.is_using_type = IsUsingType::None;
        p_data.trade_item = TradeItem::default();
        p_data.trade_money = TradeMoney::default();
        p_data.trade_status = TradeStatus::default();
    }
    send_clearisusingtype(world, storage, entity)
}

pub fn can_trade(world: &mut World, storage: &Storage, entity: GlobalKey) -> Result<bool> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        Ok(!p_data.is_using_type.inuse()
            && p_data.trade_request_entity.requesttimer <= *storage.gettick.borrow())
    } else {
        Ok(false)
    }
}
