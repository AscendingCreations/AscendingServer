use crate::{
    containers::Storage, gameloop::*, gametypes::*, maps::*, players::*, sql::*, tasks::*,
};
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
            if world.get_or_panic::<&DeathTimer>(id).0 < tick {
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
                    world.get_or_panic::<&Position>(id),
                    world.get_or_panic::<&Dir>(id).0,
                );

                //lets heal them fully on revival.
                for i in 0..VITALS_MAX {
                    let max_vital = world.get_or_panic::<&Vitals>(id);
                    {
                        world
                            .get::<&mut Vitals>(id.0)
                            .expect("Could not find Vitals")
                            .vital[i] = max_vital.vitalmax[i];
                    }
                }

                //todo: party stuff here
            }

            let killcount = world.get_or_panic::<&KillCount>(id);
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

//TODO: Add Result<(), AscendingError> to all Functions that return nothing.
pub fn player_warp(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &Entity,
    new_pos: &Position,
    dir: u8,
) {
    {
        world
            .get::<&mut Dir>(entity.0)
            .expect("Could not find Dir")
            .0 = dir;
    }
    let playerdir = world.get_or_panic::<&Dir>(entity);

    if world.get_or_panic::<&Position>(entity).map != new_pos.map {
        let old_pos = player_switch_maps(world, storage, entity, *new_pos);
        let _ = DataTaskToken::PlayerMove(old_pos.map).add_task(
            storage,
            &MovePacket::new(*entity, *new_pos, true, false, playerdir.0),
        );
        let _ = DataTaskToken::PlayerMove(new_pos.map).add_task(
            storage,
            &MovePacket::new(*entity, *new_pos, true, false, playerdir.0),
        );
        let _ = DataTaskToken::PlayerSpawn(new_pos.map)
            .add_task(storage, &PlayerSpawnPacket::new(world, entity));
        init_data_lists(world, storage, entity, old_pos.map);
        //send_weather();
    } else {
        player_swap_pos(world, storage, entity, *new_pos);
        let _ = DataTaskToken::PlayerMove(new_pos.map).add_task(
            storage,
            &MovePacket::new(*entity, *new_pos, false, false, playerdir.0),
        );
    }

    {
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .movesavecount += 1;
    }
    if world.get_or_panic::<&Player>(entity).movesavecount >= 25 {
        let _ = update_pos(&storage.pgconn.borrow(), world, entity);
        {
            world
                .get::<&mut Player>(entity.0)
                .expect("Could not find Player")
                .movesavecount = 0;
        }
    }
}

pub fn player_movement(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &Entity,
    dir: u8,
) -> bool {
    let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let player_position = world.get_or_panic::<Position>(entity);
    let mut new_pos = Position::new(
        player_position.x + adj[dir as usize].0,
        player_position.y + adj[dir as usize].1,
        player_position.map,
    );

    {
        world
            .get::<&mut Dir>(entity.0)
            .expect("Could not find Dir")
            .0 = dir;
    }

    if !new_pos.update_pos_map(storage) {
        player_warp(world, storage, entity, &player_position, dir);
        return false;
    }

    if map_path_blocked(storage, player_position, new_pos, dir) {
        player_warp(world, storage, entity, &player_position, dir);
        return false;
    }

    //TODO: Process Tile step actions here

    {
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .movesavecount += 1;
    }
    if world.get_or_panic::<&Player>(entity).movesavecount >= 25 {
        let _ = update_pos(&storage.pgconn.borrow(), world, entity);
        {
            world
                .get::<&mut Player>(entity.0)
                .expect("Could not find Player")
                .movesavecount = 0;
        }
    }

    let player_dir = world.get_or_panic::<&Dir>(entity);
    if new_pos.map != player_position.map {
        let oldpos = player_switch_maps(world, storage, entity, new_pos);
        let _ = DataTaskToken::PlayerMove(oldpos.map).add_task(
            storage,
            &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
        );
        let _ = DataTaskToken::PlayerMove(new_pos.map).add_task(
            storage,
            &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
        );
        let _ = DataTaskToken::PlayerSpawn(new_pos.map)
            .add_task(storage, &PlayerSpawnPacket::new(world, entity));

        init_data_lists(world, storage, entity, oldpos.map);
    } else {
        player_swap_pos(world, storage, entity, new_pos);
        let _ = DataTaskToken::PlayerMove(player_position.map).add_task(
            storage,
            &MovePacket::new(*entity, player_position, false, false, player_dir.0),
        );
    }

    true
}

pub fn player_earn_exp(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &Entity,
    victimlevel: i32,
    expval: i64,
    spercent: f64,
) {
    let mut giveexp = expval;

    if world.get_or_panic::<&Level>(entity).0 >= MAX_LVL as i32 || expval == 0 {
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
            .0 = *storage.gettick.borrow() + Duration::milliseconds(2000);
    }

    let leveldifference = victimlevel - world.get_or_panic::<&Level>(entity).0;

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

    while world.get_or_panic::<&Player>(entity).levelexp >= player_get_next_lvl_exp(world, entity)
        && world.get_or_panic::<&Level>(entity).0 != MAX_LVL as i32
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
                .get_or_panic::<&Player>(entity)
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
            world.get_or_panic::<&Socket>(entity).id,
            "Level Up!.".into(),
            FtlType::Level,
        );
    }

    let _ = send_vitals(world, storage, entity);
    let _ = send_level(world, storage, entity);
    let _ = DataTaskToken::PlayerVitals(world.get_or_panic::<&Position>(entity).map).add_task(
        storage,
        &VitalsPacket::new(
            *entity,
            world.get_or_panic::<&Vitals>(entity).vital,
            world.get_or_panic::<&Vitals>(entity).vitalmax,
        ),
    );
    let _ = update_level(&storage.pgconn.borrow(), world, entity);
}

pub fn player_get_next_lvl_exp(world: &mut hecs::World, entity: &Entity) -> u64 {
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

pub fn player_calc_max_hp(world: &mut hecs::World, entity: &Entity) -> i32 {
    let mut query = world
        .query_one::<&Level>(entity.0)
        .expect("player_calc_max_hp could not find query");

    if let Some(player_level) = query.get() {
        player_level.0 * 25
    } else {
        0
    }
}

pub fn player_calc_max_mp(world: &mut hecs::World, entity: &Entity) -> i32 {
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
    world: &mut hecs::World,
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
    world: &mut hecs::World,
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
    world: &mut hecs::World,
    storage: &Storage,
    entity: &Entity,
    slot: usize,
    repair_per: f32,
) {
    if let Some(item) = storage
        .bases
        .items
        .get(world.get_or_panic::<&Equipment>(entity).items[slot].num as usize)
    {
        if !item.repairable
            || world.get_or_panic::<&Equipment>(entity).items[slot].data[0]
                == world.get_or_panic::<&Equipment>(entity).items[slot].data[1]
        {
            return;
        }

        let repair_amount = (world.get_or_panic::<&Equipment>(entity).items[slot].data[0] as f32
            * repair_per) as i16;
        let repair_amount = cmp::min(
            repair_amount,
            world.get_or_panic::<&Equipment>(entity).items[slot].data[0]
                - world.get_or_panic::<&Equipment>(entity).items[slot].data[1],
        );

        {
            world
                .get::<&mut Equipment>(entity.0)
                .expect("Could not find Equipment")
                .items[slot]
                .data[0] = world.get_or_panic::<&Equipment>(entity).items[slot].data[0]
                .saturating_add(repair_amount);
        }
        //TODO: CalculateStats();

        let _ = send_equipment(world, storage, entity);
        let _ = update_equipment(&storage.pgconn.borrow(), world, entity, slot);
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
