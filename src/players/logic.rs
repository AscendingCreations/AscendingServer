use crate::{
    containers::Storage, gametypes::*, maps::can_target, npcs::npc_clear_move_path, players::*,
    socket::*, sql::*, tasks::*,
};
use chrono::Duration;
use log::debug;
use std::cmp;

use hecs::World;

pub fn update_players(world: &mut World, storage: &Storage) -> Result<()> {
    let tick = *storage.gettick.borrow();

    for id in &*storage.player_ids.borrow() {
        if world.get_or_err::<OnlineType>(id)? == OnlineType::Online {
            if world.get_or_err::<DeathType>(id)? == DeathType::Spirit {
                //timers
                if world.get_or_err::<DeathTimer>(id)?.0 < tick {
                    {
                        *world.get::<&mut DeathType>(id.0)? = DeathType::Alive;
                    }
                    send_life_status(world, storage, id, true)?;
                    player_warp(
                        world,
                        storage,
                        id,
                        &world.get_or_err::<Position>(id)?,
                        false,
                    )?;
                    init_data_lists(world, storage, id, None)?;

                    //lets heal them fully on revival.
                    for i in 0..VITALS_MAX {
                        let max_vital = world.get_or_err::<Vitals>(id)?;
                        {
                            world.get::<&mut Vitals>(id.0)?.vital[i] = max_vital.vitalmax[i];
                        }
                    }

                    //todo: party stuff here
                }

                let killcount = world.get_or_err::<KillCount>(id)?;
                if killcount.count > 0 && killcount.killcounttimer < tick {
                    {
                        world.get::<&mut KillCount>(id.0)?.count = 0;
                    }
                }
            }

            // Check Trade
            if let IsUsingType::Trading(tradeentity) = world.get_or_err::<IsUsingType>(id)? {
                if !world.contains(tradeentity.0) {
                    close_trade(world, storage, id)?;
                    send_message(
                        world,
                        storage,
                        id,
                        "Trade has cancelled the trade".into(),
                        String::new(),
                        MessageChannel::Private,
                        None,
                    )?;

                    {
                        *world.get::<&mut TradeRequestEntity>(id.0)? =
                            TradeRequestEntity::default();
                    }
                }
            };
        }
    }

    Ok(())
}

pub fn check_player_connection(world: &mut World, storage: &Storage) -> Result<()> {
    let mut remove_player_list = Vec::new();

    for (entity, timer) in world.query::<&ConnectionLoginTimer>().iter() {
        if timer.0 < *storage.gettick.borrow() {
            remove_player_list.push(entity);
        }
    }

    for i in remove_player_list.iter() {
        // Close Socket
        disconnect(Entity(*i), world, storage)?;
    }
    Ok(())
}

//If they login successfully the remove the timer from world.
pub fn send_connection_pings(world: &mut World, storage: &Storage) -> Result<()> {
    for id in &*storage.player_ids.borrow() {
        if world.get_or_err::<OnlineType>(id)? == OnlineType::Online {
            send_ping(world, storage, id)?;
        }
    }

    Ok(())
}

pub fn player_earn_exp(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    victimlevel: i32,
    expval: i64,
    spercent: f64,
) -> Result<()> {
    let mut giveexp = expval;

    if world.get_or_err::<Level>(entity)?.0 >= MAX_LVL as i32 || expval == 0 {
        return Ok(());
    }

    giveexp = (giveexp as f64 * spercent) as i64;

    {
        world.get::<&mut InCombat>(entity.0)?.0 = true;
        world.get::<&mut Combat>(entity.0)?.0 =
            *storage.gettick.borrow() + Duration::try_milliseconds(2000).unwrap_or_default();
    }

    let leveldifference = victimlevel - world.get_or_err::<Level>(entity)?.0;

    if (1..=5).contains(&leveldifference) {
        giveexp = (giveexp as f64 * 1.1) as i64;
    } else if leveldifference <= -1 {
        giveexp += (giveexp as f64 * (leveldifference as f64 * 0.1)) as i64;
    }

    {
        world.get::<&mut Player>(entity.0)?.levelexp += cmp::max(giveexp, 1) as u64;
    }

    while world.get_or_err::<Player>(entity)?.levelexp >= player_get_next_lvl_exp(world, entity)?
        && world.get_or_err::<Level>(entity)?.0 != MAX_LVL as i32
    {
        {
            world.get::<&mut Level>(entity.0)?.0 += 1;
            world.get::<&mut Player>(entity.0)?.levelexp = world
                .get_or_err::<Player>(entity)?
                .levelexp
                .saturating_sub(player_get_next_lvl_exp(world, entity)?);
            world.get::<&mut Vitals>(entity.0)?.vitalmax[VitalTypes::Hp as usize] =
                player_calc_max_hp(world, entity)?;
            world.get::<&mut Vitals>(entity.0)?.vitalmax[VitalTypes::Mp as usize] =
                player_calc_max_mp(world, entity)?;
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

        for i in 0..VitalTypes::Count as usize {
            {
                world.get::<&mut Vitals>(entity.0)?.vital[i] =
                    player_add_up_vital(world, entity, i)?;
            }
        }

        send_fltalert(
            storage,
            world.get::<&Socket>(entity.0)?.id,
            "Level Up!.".into(),
            FtlType::Level,
        )?;
    }

    send_level(world, storage, entity)?;
    DataTaskToken::PlayerVitals(world.get_or_err::<Position>(entity)?.map).add_task(
        storage,
        &VitalsPacket::new(
            *entity,
            world.get_or_err::<Vitals>(entity)?.vital,
            world.get_or_err::<Vitals>(entity)?.vitalmax,
        ),
    )?;
    update_level(storage, world, entity)
}

pub fn player_get_next_lvl_exp(world: &mut World, entity: &Entity) -> Result<u64> {
    let mut query = world.query_one::<&Level>(entity.0)?;

    if let Some(player_level) = query.get() {
        let exp_per_level = match player_level.0 {
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

        Ok(player_level.0 as u64 * exp_per_level as u64)
    } else {
        Ok(0)
    }
}

pub fn player_calc_max_hp(world: &mut World, entity: &Entity) -> Result<i32> {
    let mut query = world.query_one::<&Level>(entity.0)?;

    if let Some(player_level) = query.get() {
        Ok(player_level.0 * 25)
    } else {
        Ok(0)
    }
}

pub fn player_calc_max_mp(world: &mut World, entity: &Entity) -> Result<i32> {
    let mut query = world.query_one::<&Level>(entity.0)?;

    if let Some(player_level) = query.get() {
        Ok(player_level.0 * 25)
    } else {
        Ok(0)
    }
}

pub fn player_get_weapon_damage(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
) -> Result<(i16, i16)> {
    let mut query = world.query_one::<&mut Equipment>(entity.0)?;

    Ok(if let Some(player_equipment) = query.get() {
        let mut dmg = (0, 0);

        if player_equipment.items[EquipmentType::Weapon as usize].val > 0 {
            if let Some(item) = storage
                .bases
                .items
                .get(player_equipment.items[EquipmentType::Weapon as usize].num as usize)
            {
                dmg = (item.data[0], item.data[1]);
            }
        }

        dmg
    } else {
        (0, 0)
    })
}

pub fn player_get_armor_defense(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
) -> Result<(i16, i16)> {
    let mut query = world.query_one::<&mut Equipment>(entity.0)?;

    Ok(if let Some(player_equipment) = query.get() {
        let mut defense = (0i16, 0i16);

        for i in EquipmentType::Helmet as usize..=EquipmentType::Accessory as usize {
            if let Some(item) = storage
                .bases
                .items
                .get(player_equipment.items[i].num as usize)
            {
                defense.0 = defense.0.saturating_add(item.data[0]);
                defense.1 = defense.1.saturating_add(item.data[1]);
            }
        }

        defense
    } else {
        (0, 0)
    })
}

pub fn player_repair_equipment(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    slot: usize,
    repair_per: f32,
) -> Result<()> {
    let mut update = false;

    if let Ok(mut equipment) = world.get::<&mut Equipment>(entity.0) {
        if let Some(item) = storage.bases.items.get(equipment.items[slot].num as usize) {
            if !item.repairable || equipment.items[slot].data[0] == equipment.items[slot].data[1] {
                return Ok(());
            }

            let repair_amount = (equipment.items[slot].data[0] as f32 * repair_per) as i16;
            let repair_amount = cmp::min(
                repair_amount,
                equipment.items[slot].data[0] - equipment.items[slot].data[1],
            );

            {
                equipment.items[slot].data[0] =
                    equipment.items[slot].data[0].saturating_add(repair_amount);
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

pub fn joingame(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    let socket_id = world.get::<&Socket>(entity.0)?.id;

    {
        *world.get::<&mut OnlineType>(entity.0)? = OnlineType::Online;
    }

    // Send player index and data
    //send_myindex(storage, socket_id, entity)?;
    send_playerdata(world, storage, socket_id, entity)?;

    // Set player position based on the loaded data
    let position = world.get_or_err::<Position>(entity)?;
    player_warp(world, storage, entity, &position, true)?;

    // Add player on map
    if let Some(mapref) = storage.maps.get(&position.map) {
        let mut map = mapref.borrow_mut();
        map.add_player(storage, *entity);
        map.add_entity_to_grid(position);
    }

    send_inv(world, storage, entity)?;
    send_level(world, storage, entity)?;
    send_money(world, storage, entity)?;

    DataTaskToken::MapChat(position.map).add_task(
        storage,
        &MessagePacket::new(
            MessageChannel::Map,
            String::new(),
            format!(
                "{} has joined the game",
                world.cloned_get_or_err::<Account>(entity)?.username
            ),
            None,
        ),
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
    )
}

pub fn left_game(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    if world.get_or_err::<OnlineType>(entity)? != OnlineType::Online {
        return Ok(());
    }

    let position = world.get_or_err::<Position>(entity)?;
    DataTaskToken::MapChat(position.map).add_task(
        storage,
        &MessagePacket::new(
            MessageChannel::Map,
            String::new(),
            format!(
                "{} has left the game",
                world.cloned_get_or_err::<Account>(entity)?.username
            ),
            None,
        ),
    )?;

    update_playerdata(storage, world, entity)?;
    update_player(storage, world, entity)?;
    update_level(storage, world, entity)?;
    update_spawn(storage, world, entity)?;
    update_pos(storage, world, entity)?;
    update_currency(storage, world, entity)?;
    update_resetcount(storage, world, entity)?;

    //todo Add Update Players on map here.

    Ok(())
}

pub fn remove_all_npc_target(world: &mut World, entity: &Entity) -> Result<()> {
    let mut clear_move_path = Vec::new();
    for (entity, (worldentitytype, target)) in world
        .query::<(&WorldEntityType, &mut Target)>()
        .iter()
        .filter(|(_entity, (worldentitytype, target))| {
            let mut can_target = true;
            if **worldentitytype != WorldEntityType::Npc {
                can_target = false;
            }
            if let EntityType::Player(i, _) = target.targettype {
                if i != *entity {
                    can_target = false;
                }
            }
            can_target
        })
    {
        *target = Target::default();
        clear_move_path.push((entity, *worldentitytype));
    }

    for (entity, targettype) in clear_move_path {
        if targettype == WorldEntityType::Npc {
            npc_clear_move_path(world, &Entity(entity))?;
        }
    }
    Ok(())
}

pub fn init_trade(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<()> {
    if can_target(
        world.get_or_err::<Position>(entity)?,
        world.get_or_err::<Position>(target_entity)?,
        world.get_or_err::<DeathType>(target_entity)?,
        1,
    ) {
        if world.get_or_err::<IsUsingType>(target_entity)?.inuse()
            || world.get_or_err::<IsUsingType>(entity)?.inuse()
        {
            // ToDo Warning that other player is in trade
            return Ok(());
        }

        {
            *world.get::<&mut IsUsingType>(entity.0)? = IsUsingType::Trading(*target_entity);
            *world.get::<&mut IsUsingType>(target_entity.0)? = IsUsingType::Trading(*entity);

            *world.get::<&mut TradeItem>(entity.0)? = TradeItem::default();
            *world.get::<&mut TradeItem>(target_entity.0)? = TradeItem::default();

            *world.get::<&mut TradeMoney>(entity.0)? = TradeMoney::default();
            *world.get::<&mut TradeMoney>(target_entity.0)? = TradeMoney::default();

            *world.get::<&mut TradeStatus>(entity.0)? = TradeStatus::default();
            *world.get::<&mut TradeStatus>(target_entity.0)? = TradeStatus::default();
        }

        send_inittrade(world, storage, entity, target_entity)?;
        send_inittrade(world, storage, target_entity, entity)?;
    } else {
        // ToDo Warning not in range
    }
    Ok(())
}

pub fn process_player_trade(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<bool> {
    let entity_item = world.cloned_get_or_err::<TradeItem>(entity)?;
    let target_item = world.cloned_get_or_err::<TradeItem>(target_entity)?;
    let entity_money = world.get_or_err::<TradeMoney>(entity)?.vals;
    let target_money = world.get_or_err::<TradeMoney>(target_entity)?.vals;

    //check_temp_inv_space
    let mut entity_clone_inv = world.cloned_get_or_err::<Inventory>(entity)?;
    let mut target_clone_inv = world.cloned_get_or_err::<Inventory>(target_entity)?;
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

    Ok(true)
}

pub fn close_trade(world: &mut World, storage: &Storage, entity: &Entity) -> Result<()> {
    {
        *world.get::<&mut IsUsingType>(entity.0)? = IsUsingType::None;
        *world.get::<&mut TradeItem>(entity.0)? = TradeItem::default();
        *world.get::<&mut IsUsingType>(entity.0)? = IsUsingType::default();
        *world.get::<&mut TradeMoney>(entity.0)? = TradeMoney::default();
        *world.get::<&mut TradeStatus>(entity.0)? = TradeStatus::default();
    }
    send_clearisusingtype(world, storage, entity)
}

pub fn can_trade(world: &mut World, storage: &Storage, entity: &Entity) -> Result<bool> {
    Ok(!world.get_or_err::<IsUsingType>(entity)?.inuse()
        && world.get_or_err::<TradeRequestEntity>(entity)?.requesttimer
            <= *storage.gettick.borrow())
}
