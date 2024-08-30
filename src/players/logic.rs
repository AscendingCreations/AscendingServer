use crate::{
    containers::{GameStore, GameWorld},
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

pub async fn update_players(world: &GameWorld, storage: &GameStore) -> Result<()> {
    let tick = *storage.gettick.read().await;

    for id in &*storage.player_ids.read().await {
        if world.get_or_err::<OnlineType>(id).await? == OnlineType::Online {
            if world.get_or_err::<DeathType>(id).await? == DeathType::Spirit {
                //timers
                if world.get_or_err::<DeathTimer>(id).await?.0 < tick {
                    {
                        let lock = world.write().await;
                        *lock.get::<&mut DeathType>(id.0)? = DeathType::Alive;
                    }

                    DataTaskToken::Death(world.get_or_err::<Position>(id).await?.map)
                        .add_task(storage, death_packet(*id, DeathType::Alive)?)
                        .await?;

                    player_warp(
                        world,
                        storage,
                        id,
                        &world.get_or_err::<Spawn>(id).await?.pos,
                        false,
                    )
                    .await?;
                    //init_data_lists(world, storage, id, None)?;

                    //lets heal them fully on revival.
                    for i in 0..VITALS_MAX {
                        let max_vital = world.get_or_err::<Vitals>(id).await?;
                        {
                            let lock = world.write().await;
                            lock.get::<&mut Vitals>(id.0)?.vital[i] = max_vital.vitalmax[i];
                        }
                    }

                    //todo: party stuff here
                }

                let killcount = world.get_or_err::<KillCount>(id).await?;
                if killcount.count > 0 && killcount.killcounttimer < tick {
                    {
                        let lock = world.write().await;
                        lock.get::<&mut KillCount>(id.0)?.count = 0;
                    }
                }
            }

            // Check Trade
            if let IsUsingType::Trading(tradeentity) = world.get_or_err::<IsUsingType>(id).await? {
                if !world.contains(&tradeentity).await {
                    close_trade(world, storage, id).await?;
                    send_message(
                        world,
                        storage,
                        id,
                        "Trade has cancelled the trade".into(),
                        String::new(),
                        MessageChannel::Private,
                        None,
                    )
                    .await?;

                    {
                        let lock = world.write().await;
                        *lock.get::<&mut TradeRequestEntity>(id.0)? = TradeRequestEntity::default();
                    }
                }
            };
        }
    }

    Ok(())
}

pub async fn check_player_connection(world: &GameWorld, storage: &GameStore) -> Result<()> {
    let mut remove_player_list = Vec::new();

    {
        let lock = world.read().await;
        for (entity, timer) in lock.query::<&ConnectionLoginTimer>().iter() {
            if timer.0 < *storage.gettick.read().await {
                remove_player_list.push(entity);
            }
        }
    }

    for i in remove_player_list.iter() {
        // Close Socket
        disconnect(Entity(*i), world, storage).await?;
    }
    Ok(())
}

//If they login successfully the remove the timer from world.
pub async fn send_connection_pings(world: &GameWorld, storage: &GameStore) -> Result<()> {
    for id in &*storage.player_ids.read().await {
        if world.get_or_err::<OnlineType>(id).await? == OnlineType::Online {
            send_ping(world, storage, id).await?;
        }
    }

    Ok(())
}

pub async fn player_earn_exp(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    victimlevel: i32,
    expval: i64,
    spercent: f64,
) -> Result<()> {
    let mut giveexp = expval;

    if world.get_or_err::<Level>(entity).await?.0 >= MAX_LVL as i32 || expval == 0 {
        return Ok(());
    }

    giveexp = (giveexp as f64 * spercent) as i64;

    {
        let lock = world.write().await;
        lock.get::<&mut InCombat>(entity.0)?.0 = true;
        lock.get::<&mut Combat>(entity.0)?.0 =
            *storage.gettick.read().await + Duration::try_milliseconds(2000).unwrap_or_default();
    }

    let leveldifference = victimlevel - world.get_or_err::<Level>(entity).await?.0;

    if (1..=5).contains(&leveldifference) {
        giveexp = (giveexp as f64 * 1.1) as i64;
    } else if leveldifference <= -1 {
        giveexp += (giveexp as f64 * (leveldifference as f64 * 0.1)) as i64;
    }

    {
        let lock = world.write().await;
        lock.get::<&mut Player>(entity.0)?.levelexp += cmp::max(giveexp, 1) as u64;
    }

    while world.get_or_err::<Player>(entity).await?.levelexp
        >= player_get_next_lvl_exp(world, entity).await?
        && world.get_or_err::<Level>(entity).await?.0 != MAX_LVL as i32
    {
        {
            let lock = world.write().await;
            lock.get::<&mut Level>(entity.0)?.0 += 1;
        }

        let lvlexp = world
            .get_or_err::<Player>(entity)
            .await?
            .levelexp
            .saturating_sub(player_get_next_lvl_exp(world, entity).await?);
        let max_hp = player_calc_max_hp(world, entity).await?;
        let max_mp = player_calc_max_mp(world, entity).await?;

        {
            let lock = world.write().await;
            lock.get::<&mut Player>(entity.0)?.levelexp = lvlexp;
            lock.get::<&mut Vitals>(entity.0)?.vitalmax[VitalTypes::Hp as usize] = max_hp;
            lock.get::<&mut Vitals>(entity.0)?.vitalmax[VitalTypes::Mp as usize] = max_mp;
        }

        send_message(
            world,
            storage,
            entity,
            "You have gained a level!".into(),
            String::new(),
            MessageChannel::Private,
            None,
        )
        .await?;

        for i in 0..VitalTypes::Count as usize {
            let add_up = player_add_up_vital(world, entity, i).await?;
            let lock = world.write().await;
            lock.get::<&mut Vitals>(entity.0)?.vital[i] = add_up;
        }

        send_fltalert(
            storage,
            {
                let lock = world.read().await;
                let id = lock.get::<&Socket>(entity.0)?.id;
                id
            },
            "Level Up!.".into(),
            FtlType::Level,
        )
        .await?;
    }

    send_level(world, storage, entity).await?;
    DataTaskToken::Vitals(world.get_or_err::<Position>(entity).await?.map)
        .add_task(
            storage,
            vitals_packet(
                *entity,
                world.get_or_err::<Vitals>(entity).await?.vital,
                world.get_or_err::<Vitals>(entity).await?.vitalmax,
            )?,
        )
        .await?;
    update_level(storage, world, entity).await
}

pub async fn player_get_next_lvl_exp(world: &GameWorld, entity: &Entity) -> Result<u64> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&Level>(entity.0)?;

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

pub async fn player_calc_max_hp(world: &GameWorld, entity: &Entity) -> Result<i32> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&Level>(entity.0)?;

    if let Some(player_level) = query.get() {
        Ok(player_level.0 * 25)
    } else {
        Ok(0)
    }
}

pub async fn player_calc_max_mp(world: &GameWorld, entity: &Entity) -> Result<i32> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&Level>(entity.0)?;

    if let Some(player_level) = query.get() {
        Ok(player_level.0 * 25)
    } else {
        Ok(0)
    }
}

pub async fn player_get_weapon_damage(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
) -> Result<(i16, i16)> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Equipment>(entity.0)?;

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

pub async fn player_get_armor_defense(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
) -> Result<(i16, i16)> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Equipment>(entity.0)?;

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

pub async fn player_repair_equipment(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    slot: usize,
    repair_per: f32,
) -> Result<()> {
    let mut update = false;
    {
        let lock = world.write().await;
        let equipment = lock.get::<&mut Equipment>(entity.0);
        if let Ok(mut equipment) = equipment {
            if let Some(item) = storage.bases.items.get(equipment.items[slot].num as usize) {
                if !item.repairable
                    || equipment.items[slot].data[0] == equipment.items[slot].data[1]
                {
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
    }

    if update {
        send_equipment(world, storage, entity).await?;
        update_equipment(storage, world, entity, slot).await?;
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

pub async fn joingame(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    let socket_id = {
        let lock = world.write().await;
        let socket_id = lock.get::<&Socket>(entity.0)?.id;
        *lock.get::<&mut OnlineType>(entity.0)? = OnlineType::Online;
        socket_id
    };

    // Send player index and data
    //send_myindex(storage, socket_id, entity)?;
    send_playerdata(world, storage, socket_id, entity).await?;

    // Set player position based on the loaded data
    let position = world.get_or_err::<Position>(entity).await?;
    player_warp(world, storage, entity, &position, true).await?;

    // Add player on map
    if let Some(mapref) = storage.maps.get(&position.map) {
        let mut map = mapref.write().await;
        map.add_player(storage, *entity).await;
        map.add_entity_to_grid(position);
    }

    send_inv(world, storage, entity).await?;
    send_level(world, storage, entity).await?;
    send_money(world, storage, entity).await?;

    DataTaskToken::MapChat(position.map)
        .add_task(
            storage,
            message_packet(
                MessageChannel::Map,
                String::new(),
                format!(
                    "{} has joined the game",
                    world.cloned_get_or_err::<Account>(entity).await?.username
                ),
                None,
            )?,
        )
        .await?;

    debug!("Login Ok");
    // Finish loading
    send_loginok(storage, socket_id).await?;
    send_message(
        world,
        storage,
        entity,
        "Welcome Message".into(),
        String::new(),
        MessageChannel::Private,
        None,
    )
    .await
}

pub async fn left_game(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    if world.get_or_err::<OnlineType>(entity).await? != OnlineType::Online {
        return Ok(());
    }

    let position = world.get_or_err::<Position>(entity).await?;
    DataTaskToken::MapChat(position.map)
        .add_task(
            storage,
            message_packet(
                MessageChannel::Map,
                String::new(),
                format!(
                    "{} has left the game",
                    world.cloned_get_or_err::<Account>(entity).await?.username
                ),
                None,
            )?,
        )
        .await?;

    update_playerdata(storage, world, entity).await?;
    update_player(storage, world, entity).await?;
    update_level(storage, world, entity).await?;
    update_spawn(storage, world, entity).await?;
    update_pos(storage, world, entity).await?;
    update_currency(storage, world, entity).await?;
    update_resetcount(storage, world, entity).await?;

    //todo Add Update Players on map here.

    Ok(())
}

pub async fn remove_all_npc_target(world: &GameWorld, entity: &Entity) -> Result<()> {
    let mut clear_move_path = Vec::new();
    {
        let lock = world.read().await;
        for (entity, (worldentitytype, target)) in lock
            .query::<(&WorldEntityType, &mut Target)>()
            .iter()
            .filter(|(_entity, (worldentitytype, target))| {
                let mut can_target = true;
                if **worldentitytype != WorldEntityType::Npc {
                    can_target = false;
                }
                if let EntityType::Player(i, _) = target.target_type {
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
    }

    for (entity, targettype) in clear_move_path {
        if targettype == WorldEntityType::Npc {
            npc_clear_move_path(world, &Entity(entity)).await?;
        }
    }
    Ok(())
}

pub async fn init_trade(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<()> {
    if can_target(
        world.get_or_err::<Position>(entity).await?,
        world.get_or_err::<Position>(target_entity).await?,
        world.get_or_err::<DeathType>(target_entity).await?,
        1,
    ) {
        if world
            .get_or_err::<IsUsingType>(target_entity)
            .await?
            .inuse()
            || world.get_or_err::<IsUsingType>(entity).await?.inuse()
        {
            // ToDo Warning that other player is in trade
            return Ok(());
        }

        {
            let lock = world.write().await;
            *lock.get::<&mut IsUsingType>(entity.0)? = IsUsingType::Trading(*target_entity);
            *lock.get::<&mut IsUsingType>(target_entity.0)? = IsUsingType::Trading(*entity);

            *lock.get::<&mut TradeItem>(entity.0)? = TradeItem::default();
            *lock.get::<&mut TradeItem>(target_entity.0)? = TradeItem::default();

            *lock.get::<&mut TradeMoney>(entity.0)? = TradeMoney::default();
            *lock.get::<&mut TradeMoney>(target_entity.0)? = TradeMoney::default();

            *lock.get::<&mut TradeStatus>(entity.0)? = TradeStatus::default();
            *lock.get::<&mut TradeStatus>(target_entity.0)? = TradeStatus::default();
        }

        send_inittrade(world, storage, entity, target_entity).await?;
        send_inittrade(world, storage, target_entity, entity).await?;
    } else {
        // ToDo Warning not in range
    }
    Ok(())
}

pub async fn process_player_trade(
    world: &GameWorld,
    storage: &GameStore,
    entity: &Entity,
    target_entity: &Entity,
) -> Result<bool> {
    let entity_item = world.cloned_get_or_err::<TradeItem>(entity).await?;
    let target_item = world.cloned_get_or_err::<TradeItem>(target_entity).await?;
    let entity_money = world.get_or_err::<TradeMoney>(entity).await?.vals;
    let target_money = world.get_or_err::<TradeMoney>(target_entity).await?.vals;

    //check_temp_inv_space
    let mut entity_clone_inv = world.cloned_get_or_err::<Inventory>(entity).await?;
    let mut target_clone_inv = world.cloned_get_or_err::<Inventory>(target_entity).await?;

    for item in entity_item.items.clone().iter_mut() {
        if item.val > 0 && !check_temp_inv_space(storage, item, &mut target_clone_inv).await? {
            return Ok(false);
        }
    }

    for item in target_item.items.clone().iter_mut() {
        if item.val > 0 && !check_temp_inv_space(storage, item, &mut entity_clone_inv).await? {
            return Ok(false);
        }
    }

    for item in entity_item.items.iter() {
        if item.val > 0 {
            take_inv_items(world, storage, entity, item.num, item.val).await?;
        }
    }

    player_take_vals(world, storage, entity, entity_money).await?;

    for item in target_item.items.iter() {
        if item.val > 0 {
            take_inv_items(world, storage, target_entity, item.num, item.val).await?;
        }
    }

    player_take_vals(world, storage, target_entity, target_money).await?;

    for item in entity_item.items.clone().iter_mut() {
        if item.val > 0 {
            give_inv_item(world, storage, target_entity, item).await?;
        }
    }

    player_give_vals(world, storage, target_entity, entity_money).await?;

    for item in target_item.items.clone().iter_mut() {
        if item.val > 0 {
            give_inv_item(world, storage, entity, item).await?;
        }
    }

    player_give_vals(world, storage, entity, target_money).await?;

    Ok(true)
}

pub async fn close_trade(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<()> {
    {
        let lock = world.write().await;
        *lock.get::<&mut IsUsingType>(entity.0)? = IsUsingType::None;
        *lock.get::<&mut TradeItem>(entity.0)? = TradeItem::default();
        *lock.get::<&mut IsUsingType>(entity.0)? = IsUsingType::default();
        *lock.get::<&mut TradeMoney>(entity.0)? = TradeMoney::default();
        *lock.get::<&mut TradeStatus>(entity.0)? = TradeStatus::default();
    }
    send_clearisusingtype(world, storage, entity).await
}

pub async fn can_trade(world: &GameWorld, storage: &GameStore, entity: &Entity) -> Result<bool> {
    Ok(!world.get_or_err::<IsUsingType>(entity).await?.inuse()
        && world
            .get_or_err::<TradeRequestEntity>(entity)
            .await?
            .requesttimer
            <= *storage.gettick.read().await)
}
