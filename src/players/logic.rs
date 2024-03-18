use crate::{containers::Storage, gameloop::*, gametypes::*, players::*, sql::*, tasks::*};
use chrono::Duration;
use std::cmp;

use hecs::World;

pub fn update_players(world: &mut World, storage: &Storage) {
    let tick = *storage.gettick.borrow();

    for id in &*storage.player_ids.borrow() {
        if world.get_or_panic::<OnlineType>(id) == OnlineType::Online
            && world.get_or_panic::<DeathType>(id) == DeathType::Spirit
        {
            //timers
            if world.get_or_panic::<DeathTimer>(id).0 < tick {
                {
                    *world
                        .get::<&mut DeathType>(id.0)
                        .expect("Could not find DeathType") = DeathType::Alive;
                }
                let _ = send_life_status(world, storage, id, true);
                player_warp(
                    world,
                    storage,
                    id,
                    &world.get_or_panic::<Position>(id),
                    false,
                );

                //lets heal them fully on revival.
                for i in 0..VITALS_MAX {
                    let max_vital = world.get_or_panic::<Vitals>(id);
                    {
                        world
                            .get::<&mut Vitals>(id.0)
                            .expect("Could not find Vitals")
                            .vital[i] = max_vital.vitalmax[i];
                    }
                }

                //todo: party stuff here
            }

            let killcount = world.get_or_panic::<KillCount>(id);
            if killcount.count > 0 && killcount.killcounttimer < tick {
                {
                    world
                        .get::<&mut KillCount>(id.0)
                        .expect("Could not find KillCount")
                        .count = 0;
                }
            }
        }
    }
}

pub fn player_earn_exp(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    victimlevel: i32,
    expval: i64,
    spercent: f64,
) {
    let mut giveexp = expval;

    if world.get_or_panic::<Level>(entity).0 >= MAX_LVL as i32 || expval == 0 {
        return;
    }

    giveexp = (giveexp as f64 * spercent) as i64;

    {
        world
            .get::<&mut InCombat>(entity.0)
            .expect("Could not find InCombat")
            .0 = true;
        world
            .get::<&mut Combat>(entity.0)
            .expect("Could not find Combat")
            .0 = *storage.gettick.borrow() + Duration::try_milliseconds(2000).unwrap_or_default();
    }

    let leveldifference = victimlevel - world.get_or_panic::<Level>(entity).0;

    if (1..=5).contains(&leveldifference) {
        giveexp = (giveexp as f64 * 1.1) as i64;
    } else if leveldifference <= -1 {
        giveexp += (giveexp as f64 * (leveldifference as f64 * 0.1)) as i64;
    }

    {
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .levelexp = cmp::max(giveexp, 1) as u64;
    }

    while world.get_or_panic::<Player>(entity).levelexp >= player_get_next_lvl_exp(world, entity)
        && world.get_or_panic::<Level>(entity).0 != MAX_LVL as i32
    {
        {
            world
                .get::<&mut Level>(entity.0)
                .expect("Could not find Vitals")
                .0 += 1;
            world
                .get::<&mut Player>(entity.0)
                .expect("Could not find Player")
                .levelexp = world
                .get_or_panic::<Player>(entity)
                .levelexp
                .saturating_sub(player_get_next_lvl_exp(world, entity));
            world
                .get::<&mut Vitals>(entity.0)
                .expect("Could not find Vitals")
                .vitalmax[VitalTypes::Hp as usize] = player_calc_max_hp(world, entity);
            world
                .get::<&mut Vitals>(entity.0)
                .expect("Could not find Vitals")
                .vitalmax[VitalTypes::Mp as usize] = player_calc_max_mp(world, entity);
        }

        for i in 0..VitalTypes::Count as usize {
            {
                world
                    .get::<&mut Vitals>(entity.0)
                    .expect("Could not find Vitals")
                    .vital[i] = player_add_up_vital(world, entity, i);
            }
        }

        let _ = send_fltalert(
            storage,
            world.get::<&Socket>(entity.0).unwrap().id,
            "Level Up!.".into(),
            FtlType::Level,
        );
    }

    let _ = send_vitals(world, storage, entity);
    let _ = send_level(world, storage, entity);
    let _ = DataTaskToken::PlayerVitals(world.get_or_panic::<Position>(entity).map).add_task(
        storage,
        &VitalsPacket::new(
            *entity,
            world.get_or_panic::<Vitals>(entity).vital,
            world.get_or_panic::<Vitals>(entity).vitalmax,
        ),
    );
    let _ = update_level(storage, world, entity);
}

pub fn player_get_next_lvl_exp(world: &mut World, entity: &Entity) -> u64 {
    let mut query = world
        .query_one::<&Level>(entity.0)
        .expect("player_get_next_lvl_exp could not find query");

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

        player_level.0 as u64 * exp_per_level as u64
    } else {
        0
    }
}

pub fn player_calc_max_hp(world: &mut World, entity: &Entity) -> i32 {
    let mut query = world
        .query_one::<&Level>(entity.0)
        .expect("player_calc_max_hp could not find query");

    if let Some(player_level) = query.get() {
        player_level.0 * 25
    } else {
        0
    }
}

pub fn player_calc_max_mp(world: &mut World, entity: &Entity) -> i32 {
    let mut query = world
        .query_one::<&Level>(entity.0)
        .expect("player_calc_max_mp could not find query");

    if let Some(player_level) = query.get() {
        player_level.0 * 25
    } else {
        0
    }
}

pub fn player_get_weapon_damage(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
) -> (i16, i16) {
    let mut query = world
        .query_one::<&mut Equipment>(entity.0)
        .expect("player_get_weapon_damage could not find query");

    if let Some(player_equipment) = query.get() {
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
    }
}

pub fn player_get_armor_defense(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
) -> (i16, i16) {
    let mut query = world
        .query_one::<&mut Equipment>(entity.0)
        .expect("player_get_armor_defense could not find query");

    if let Some(player_equipment) = query.get() {
        let mut defense = (0i16, 0i16);

        for i in EquipmentType::Helmet as usize..=EquipmentType::Accessory2 as usize {
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
    }
}

pub fn player_repair_equipment(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    slot: usize,
    repair_per: f32,
) {
    let mut update = false;

    if let Ok(mut equipment) = world.get::<&mut Equipment>(entity.0) {
        if let Some(item) = storage.bases.items.get(equipment.items[slot].num as usize) {
            if !item.repairable || equipment.items[slot].data[0] == equipment.items[slot].data[1] {
                return;
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
            //TODO: CalculateStats();

            update = true;
        }
    }

    //Sherwin: We seperated these so the Reference can get unloaded after being used. Then we can use it again.
    if update {
        let _ = send_equipment(world, storage, entity);
        let _ = update_equipment(storage, world, entity, slot);
    }
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

pub fn get_damage_percentage(damage: u32, hp: (u32, u32)) -> f64 {
    let curhp = cmp::min(hp.0, hp.1);
    let abs_damage = cmp::min(damage, curhp) as f64;
    abs_damage / curhp as f64
}

pub fn joingame(world: &mut World, storage: &Storage, entity: &Entity) {
    let socket_id = world.get::<&Socket>(entity.0).unwrap().id;

    {
        *world.get::<&mut OnlineType>(entity.0).unwrap() = OnlineType::Online;
    }

    // Send player index and data
    let _ = send_myindex(storage, socket_id, entity);
    let _ = send_playerdata(world, storage, socket_id, entity);

    // Set player position based on the loaded data
    let position = world.get_or_panic::<Position>(entity);
    player_warp(world, storage, entity, &position, true);

    // Add player on map
    if let Some(mapref) = storage.maps.get(&position.map) {
        let mut map = mapref.borrow_mut();
        map.add_player(storage, *entity);
        map.add_entity_to_grid(position);
    }

    println!("Login Ok");
    // Finish loading
    let _ = send_loginok(storage, socket_id);
}
