use crate::{
    containers::Storage, gameloop::*, gametypes::*, maps::*, players::*, sql::*, tasks::*,
};
use chrono::Duration;
use std::cmp;

use hecs::World;

pub fn update_players(world: &mut World, storage: &Storage) {
    let tick = *storage.gettick.borrow();

    for (entity, (_, 
            (deathtimer, 
            life, 
            dir, 
            position,
            killcount,
            vitals))) in world
        .query::<((&WorldEntityType, &OnlineType), 
            (&DeathTimer, 
            &mut DeathType, 
            &Dir, 
            &Position,
            &mut KillCount,
            &mut Vitals))>()
        .iter()
        .filter(|(_entity, 
            ((worldentitytype, onlinetype), _))| {
            **worldentitytype == WorldEntityType::Player && **onlinetype == OnlineType::Online
        })
    {
        if *life == DeathType::Spirit {
            //timers
            if deathtimer.0 < tick {
                *life = DeathType::Alive;
                let _ = send_life_status(world, storage, &Entity(entity), true);
                player_warp(world, storage, &Entity(entity), &position, dir.0);

                //lets heal them fully on revival.
                for i in 0..VITALS_MAX {
                    vitals.vital[i] = vitals.vitalmax[i];
                }

                //todo: party stuff here
            }

            if killcount.count > 0 && killcount.killcounttimer < tick {
                killcount.count = 0;
            }
        }
    }
}

//TODO: Add Result<(), AscendingError> to all Functions that return nothing.
pub fn player_warp(world: &mut hecs::World, storage: &Storage, entity: &Entity, new_pos: &Position, dir: u8) {
    if let Ok(mut result) = 
        world.query_one::<(
            &mut Dir,
            &mut Position,
            &mut Player)>(entity.0) {
        if let Some((player_dir,
            player_position,
            player_data)) = result.get() {
        
            player_dir.0 = dir;

            if player_position.map != new_pos.map {
                let old_pos = player_switch_maps(world, storage, entity, new_pos.clone());
                let _ = DataTaskToken::PlayerMove(old_pos.map).add_task(
                    world, storage, 
                    &MovePacket::new(*entity, *new_pos, true, false, player_dir.0),
                );
                let _ = DataTaskToken::PlayerMove(new_pos.map).add_task(
                    world, storage, 
                    &MovePacket::new(*entity, *new_pos, true, false, player_dir.0),
                );
                let _ = DataTaskToken::PlayerSpawn(new_pos.map)
                    .add_task(world, storage,
                        &PlayerSpawnPacket::new(world, entity));
                init_data_lists(world, storage, entity, old_pos.map);
                //send_weather();
            } else {
                player_swap_pos(world, storage, entity, new_pos.clone());
                let _ = DataTaskToken::PlayerMove(new_pos.map).add_task(
                    world, storage, 
                    &MovePacket::new(*entity, *new_pos, false, false, player_dir.0),
                );
            }

            player_data.movesavecount += 1;
            if player_data.movesavecount >= 25 {
                let _ = update_pos(&mut storage.pgconn.borrow_mut(), world, entity);
                player_data.movesavecount = 0;
            }
        }
    }
}

pub fn player_movement(world: &mut hecs::World, storage: &Storage, entity: &Entity, dir: u8) -> bool {
    if let Ok(mut result) = 
        world.query_one::<(
            &mut Dir,
            &mut Position,
            &mut Player)>(entity.0) {
        if let Some((player_dir,
            player_position,
            player_data)) = result.get() {
            
            let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
            let mut new_pos = Position::new(
                player_position.x + adj[dir as usize].0,
                player_position.y + adj[dir as usize].1,
                player_position.map,
            );

            player_dir.0 = dir;

            if !new_pos.update_pos_map(world, storage) {
                player_warp(world, storage, entity, &player_position, dir);
                return false;
            }

            if map_path_blocked(storage, *player_position, new_pos, dir) {
                player_warp(world, storage, entity, &player_position, dir);
                return false;
            }

            //TODO: Process Tile step actions here

            player_data.movesavecount += 1;

            if player_data.movesavecount >= 25 {
                let _ = update_pos(&mut storage.pgconn.borrow_mut(), world, entity);
                player_data.movesavecount = 0;
            }

            if new_pos.map != player_position.map {
                let oldpos = player_switch_maps(world, storage, entity, new_pos);
                let _ = DataTaskToken::PlayerMove(oldpos.map).add_task(
                    world, storage,
                    &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
                );
                let _ = DataTaskToken::PlayerMove(new_pos.map).add_task(
                    world, storage,
                    &MovePacket::new(*entity, new_pos, false, true, player_dir.0),
                );
                let _ = DataTaskToken::PlayerSpawn(new_pos.map)
                    .add_task(world, storage, &PlayerSpawnPacket::new(world, entity));

                init_data_lists(world, storage, entity, oldpos.map);
            } else {
                player_swap_pos(world, storage, entity, new_pos);
                let _ = DataTaskToken::PlayerMove(player_position.map).add_task(
                    world, storage, 
                    &MovePacket::new(*entity, *player_position, false, false, player_dir.0),
                );
            }

            return true;
        }
    }
    false
}

pub fn player_earn_exp(
    world: &mut hecs::World,
    storage: &Storage,
    entity: &Entity,
    victimlevel: i32,
    expval: i64,
    spercent: f64,
) {
    if let Ok(mut result) = 
        world.query_one::<(
            &mut Vitals,
            &mut Player,
            &mut Position,
            &mut Level,
            &mut Combat,
            &mut InCombat,
            &Socket,)>(entity.0) {
        if let Some((player_vital,
            player_data,
            player_position,
            player_level,
            combat_timer,
            in_combat,
            Socket,)) = result.get() {
            
            let mut giveexp = expval;

            if player_level.0 >= MAX_LVL as i32 || expval == 0 {
                return;
            }

            giveexp = (giveexp as f64 * spercent) as i64;

            in_combat.0 = true;
            combat_timer.0 = *storage.gettick.borrow() + Duration::milliseconds(2000);

            let leveldifference = victimlevel - player_level.0;

            if (1..=5).contains(&leveldifference) {
                giveexp = (giveexp as f64 * 1.1) as i64;
            } else if leveldifference <= -1 {
                giveexp += (giveexp as f64 * (leveldifference as f64 * 0.1)) as i64;
            }

            player_data.levelexp += cmp::max(giveexp, 1) as u64;

            while player_data.levelexp >= player_get_next_lvl_exp(world, entity) && player_level.0 != MAX_LVL as i32 {
                player_level.0 += 1;
                player_data.levelexp = player_data.levelexp.saturating_sub(player_get_next_lvl_exp(world, entity));
                player_vital.vitalmax[VitalTypes::Hp as usize] = player_calc_max_hp(world, entity);
                player_vital.vitalmax[VitalTypes::Mp as usize] = player_calc_max_mp(world, entity);

                for i in 0..VitalTypes::Count as usize {
                    player_vital.vital[i] = player_add_up_vital(world, entity, i);
                }

                let _ = send_fltalert(storage, Socket.id, "Level Up!.".into(), FtlType::Level);
            }

            let _ = send_vitals(world, storage, entity);
            let _ = send_level(world, storage, entity);
            let _ = DataTaskToken::PlayerVitals(player_position.map).add_task(
                world, storage,
                &VitalsPacket::new(*entity, player_vital.vital, player_vital.vitalmax),
            );
            let _ = update_level(&mut storage.pgconn.borrow_mut(), world, entity);
        }
    }
}

pub fn player_get_next_lvl_exp(world: &mut hecs::World, entity: &Entity) -> u64 {
    let mut query = 
        world.query_one::<&Level>(entity.0).expect("player_get_next_lvl_exp could not find query");
    
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
    let mut query = 
        world.query_one::<&Level>(entity.0).expect("player_calc_max_hp could not find query");
    
    if let Some(player_level) = query.get() {
        player_level.0 * 25
    } else {
        0
    }
}

pub fn player_calc_max_mp(world: &mut hecs::World, entity: &Entity) -> i32 {
    let mut query = 
        world.query_one::<&Level>(entity.0).expect("player_calc_max_mp could not find query");
    
    if let Some(player_level) = query.get() {
        player_level.0 * 25
    } else {
        0
    }
}

pub fn player_get_weapon_damage(world: &mut hecs::World, storage: &Storage, entity: &Entity) -> (i16, i16) {
    let mut query = 
        world.query_one::<&mut Equipment>(entity.0).expect("player_get_weapon_damage could not find query");

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

pub fn player_get_armor_defense(world: &mut hecs::World, storage: &Storage, entity: &Entity) -> (i16, i16) {
    let mut query = 
        world.query_one::<&mut Equipment>(entity.0).expect("player_get_armor_defense could not find query");
    
    if let Some(player_equipment) = query.get() {
        let mut defense = (0i16, 0i16);

        for i in EquipmentType::Helmet as usize..=EquipmentType::Accessory2 as usize {
            if let Some(item) = storage
                .bases
                .items
                .get(player_equipment.items[i].num as usize) {
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
    if let Ok(mut result) = 
        world.query_one::<&mut Equipment>(entity.0) {
        if let Some(player_equipment) = result.get() {
            if let Some(item) = storage
                .bases
                .items
                .get(player_equipment.items[slot].num as usize) {
                if !item.repairable || player_equipment.items[slot].data[0] == player_equipment.items[slot].data[1] {
                    return;
                }

                let repair_amount = (player_equipment.items[slot].data[0] as f32 * repair_per) as i16;
                let repair_amount = cmp::min(
                    repair_amount,
                    player_equipment.items[slot].data[0] - player_equipment.items[slot].data[1],
                );

                player_equipment.items[slot].data[0] = player_equipment.items[slot].data[0].saturating_add(repair_amount);
                //TODO: CalculateStats();

                let _ = send_equipment(world, storage, entity);
                let _ = update_equipment(&mut storage.pgconn.borrow_mut(), world, entity, slot);
            }
        }
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