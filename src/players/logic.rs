use crate::{
    containers::Storage, gameloop::*, gametypes::*, maps::*, players::*, sql::*, tasks::*,
};
use chrono::Duration;
use std::cmp;

pub fn update_players(world: &Storage) {
    let tick = *world.gettick.borrow();

    for i in &*world.player_ids.borrow() {
        if let Some(player) = world.players.borrow().get(*i) {
            let mut player = player.borrow_mut();

            if player.e.life.is_spirit() {
                //timers
                if player.e.deathtimer < tick {
                    player.e.life = DeathType::Alive;
                    let _ = send_life_status(world, &player, true);
                    let spawn = player.e.spawn;
                    let dir = player.e.dir;

                    player.warp(world, spawn, dir);

                    //lets heal them fully on revival.
                    for i in 0..VITALS_MAX {
                        player.e.vital[i] = player.e.vitalmax[i];
                    }

                    //todo: party stuff here
                }

                //remove kill count after a while this is used to cut experiance from campers.
                if player.e.killcount > 0 && player.e.killcounttimer < tick {
                    player.e.killcount = 0;
                }
            }
        }
    }
}

impl Player {
    pub fn warp(&mut self, world: &Storage, new_pos: Position, dir: u8) {
        self.e.dir = dir;

        if self.e.pos.map != new_pos.map {
            let oldpos = self.switch_maps(world, new_pos);
            let _ = send_mapswitch(world, self, oldpos.map, true);
            init_data_lists(world, self, oldpos.map);
            //send_weather();
        } else {
            self.swap_pos(world, new_pos);
            let _ = send_move(world, self, true);
        }

        self.movesavecount += 1;

        if self.movesavecount >= 25 {
            let _ = update_pos(&mut world.pgconn.borrow_mut(), self);
            self.movesavecount = 0;
        }
    }

    pub fn movement(&mut self, world: &Storage, dir: u8) -> bool {
        let adj = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let mut new_pos = Position::new(
            self.e.pos.x + adj[dir as usize].0,
            self.e.pos.y + adj[dir as usize].1,
            self.e.pos.map,
        );

        self.e.dir = dir;

        if !new_pos.update_pos_map(world) {
            self.warp(world, self.e.pos, dir);
            return false;
        }

        if map_path_blocked(world, self.e.pos, new_pos, dir) {
            self.warp(world, self.e.pos, dir);
            return false;
        }

        //TODO: Process Tile step actions here

        self.movesavecount += 1;

        if self.movesavecount >= 25 {
            let _ = update_pos(&mut world.pgconn.borrow_mut(), self);
            self.movesavecount = 0;
        }

        if new_pos.map != self.e.pos.map {
            let oldpos = self.switch_maps(world, new_pos);
            let _ = send_mapswitch(world, self, oldpos.map, false);
            init_data_lists(world, self, oldpos.map);
        } else {
            self.swap_pos(world, new_pos);
            let _ = send_move(world, self, false);
        }

        true
    }

    pub fn earn_exp(&mut self, world: &Storage, victimlevel: i32, expval: i64, spercent: f64) {
        let mut giveexp = expval;

        if self.e.level >= MAX_LVL as i32 || expval == 0 {
            return;
        }

        giveexp = (giveexp as f64 * spercent) as i64;

        self.e.incombat = true;
        self.e.combattimer = *world.gettick.borrow() + Duration::milliseconds(2000);

        let leveldifference = victimlevel - self.e.level;

        if (1..=5).contains(&leveldifference) {
            giveexp = (giveexp as f64 * 1.1) as i64;
        } else if leveldifference <= -1 {
            giveexp += (giveexp as f64 * (leveldifference as f64 * 0.1)) as i64;
        }

        self.levelexp += cmp::max(giveexp, 1) as u64;

        while self.levelexp >= self.get_next_lvl_exp() && self.e.level != MAX_LVL as i32 {
            self.e.level += 1;
            self.levelexp = self.levelexp.saturating_sub(self.get_next_lvl_exp());
            self.e.vitalmax[VitalTypes::Hp as usize] = self.calc_max_hp();
            self.e.vitalmax[VitalTypes::Mp as usize] = self.calc_max_mp();

            for i in 0..VitalTypes::Count as usize {
                self.e.vital[i] = self.e.add_up_vital(i);
            }

            let _ = send_fltalert(world, self.socket_id, "Level Up!.".into(), FtlType::Level);
        }

        let _ = send_vitals(world, self);
        let _ = send_level(world, self);
        let _ = update_level(&mut world.pgconn.borrow_mut(), self);
        let _ = update_level(&mut world.pgconn.borrow_mut(), self);
    }

    pub fn get_next_lvl_exp(&self) -> u64 {
        let exp_per_level = match self.e.level {
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

        self.e.level as u64 * exp_per_level as u64
    }

    pub fn calc_max_hp(&self) -> i32 {
        self.e.level * 25
    }

    pub fn calc_max_mp(&self) -> i32 {
        self.e.level * 25
    }

    pub fn get_weapon_damage(&self, world: &Storage) -> (i16, i16) {
        let mut dmg = (0, 0);

        if self.equip[EquipmentType::Weapon as usize].val > 0 {
            if let Some(item) = world
                .bases
                .item
                .get(self.equip[EquipmentType::Weapon as usize].num as usize)
            {
                dmg = (item.data[0], item.data[1]);
            }
        }

        dmg
    }

    pub fn get_armor_defense(&self, world: &Storage) -> (i16, i16) {
        let mut defense = (0i16, 0i16);

        for i in EquipmentType::Helmet as usize..=EquipmentType::Accessory2 as usize {
            if let Some(item) = world.bases.item.get(self.equip[i].num as usize) {
                defense.0 = defense.0.saturating_add(item.data[0]);
                defense.1 = defense.1.saturating_add(item.data[1]);
            }
        }

        defense
    }

    pub fn repair_equipment(&mut self, world: &Storage, slot: usize, repair_per: f32) {
        if let Some(item) = world.bases.item.get(self.equip[slot].num as usize) {
            if !item.repairable || self.equip[slot].data[0] == self.equip[slot].data[1] {
                return;
            }

            let repair_amount = (self.equip[slot].data[0] as f32 * repair_per) as i16;
            let repair_amount = cmp::min(
                repair_amount,
                self.equip[slot].data[0] - self.equip[slot].data[1],
            );

            self.equip[slot].data[0] = self.equip[slot].data[0].saturating_add(repair_amount);
            //TODO: CalculateStats();

            let _ = send_equipment(world, self);
            let _ = update_equipment(&mut world.pgconn.borrow_mut(), self, slot);
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
