use crate::{containers::SALT, gametypes::*, sql::*, time_ext::*};
use argon2::{Argon2, PasswordHasher};
use password_hash::SaltString;
use std::convert::TryInto;

#[derive(Queryable, Identifiable, Debug, PartialEq, Eq)]
#[primary_key(uid)]
#[table_name = "players"]
pub struct PlayerWithPassword {
    pub uid: i64,
    pub password: String,
}

#[derive(Debug, Queryable, Insertable)]
#[table_name = "players"]
pub struct PGPlayer {
    name: String,
    address: String,
    sprite: i16,
    spawn: Position,
    itemtimer: MyInstant,
    vals: i64,
    data: Vec<i64>,
    access: UserAccess,
    sstats: Vec<i16>,
    sstatbuffs: Vec<i16>,
    sstatexp: Vec<i64>,
    cstats: Vec<i16>,
    cstatbuffs: Vec<i16>,
    cstatexp: Vec<i64>,
    passresetcode: Option<String>,
    pos: Position,
    vital: Vec<i32>,
    deathtimer: MyInstant,
    indeath: bool,
    email: String,
    password: String,
    username: String,
    level: i32,
    levelexp: i64,
    resetcount: i16,
    pk: bool,
}

impl PGPlayer {
    pub fn new(
        user: &crate::players::Player,
        username: String,
        email: String,
        password: String,
    ) -> PGPlayer {
        let argon = Argon2::default();
        let hashed_password = if let Ok(salt) = SaltString::b64_encode(SALT) {
            if let Ok(hash) = argon.hash_password(password.as_bytes(), &salt) {
                hash.to_string()
            } else {
                String::from("FailedPasswordHash")
            }
        } else {
            String::from("FailedPasswordHash")
        };

        PGPlayer {
            name: user.name.clone(),
            address: user.addr.clone(),
            sprite: user.sprite as i16,
            spawn: user.spawn,
            itemtimer: user.itemtimer,
            vals: user.vals as i64,
            data: user.data.to_vec(),
            access: user.access,
            sstats: user.sstats.to_vec(),
            sstatbuffs: user.sstatbuffs.to_vec(),
            sstatexp: user.sstatexp.iter().map(|x| *x as i64).collect(),
            cstats: user.e.cstat.to_vec(),
            cstatbuffs: user.e.buffs.to_vec(),
            cstatexp: user.cstatexp.iter().map(|x| *x as i64).collect(),
            passresetcode: None,
            pos: user.e.pos,
            vital: user.e.vital.to_vec(),
            deathtimer: user.e.deathtimer,
            indeath: user.e.life.is_spirit(),
            email,
            password: hashed_password,
            username,
            level: user.e.level,
            levelexp: user.levelexp as i64,
            resetcount: user.resetcount,
            pk: user.pk,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Queryable, Insertable, Identifiable)]
#[table_name = "player_ret"]
#[primary_key(uid)]
pub struct PGPlayerWithID {
    uid: i64,
    name: String,
    address: String,
    sprite: i16,
    spawn: Position,
    itemtimer: MyInstant,
    vals: i64,
    data: Vec<i64>,
    access: UserAccess,
    sstats: Vec<i16>,
    sstatbuffs: Vec<i16>,
    sstatexp: Vec<i64>,
    cstats: Vec<i16>,
    cstatbuffs: Vec<i16>,
    cstatexp: Vec<i64>,
    pos: Position,
    vital: Vec<i32>,
    deathtimer: MyInstant,
    indeath: bool,
    level: i32,
    levelexp: i64,
    resetcount: i16,
    pk: bool,
}

impl PGPlayerWithID {
    pub fn new(user: &crate::players::Player) -> PGPlayerWithID {
        PGPlayerWithID {
            uid: user.accid,
            name: user.name.clone(),
            address: user.addr.clone(),
            sprite: user.sprite as i16,
            spawn: user.spawn,
            itemtimer: user.itemtimer,
            vals: user.vals as i64,
            data: user.data.to_vec(),
            access: user.access,
            sstats: user.sstats.to_vec(),
            sstatbuffs: user.sstatbuffs.to_vec(),
            sstatexp: user.sstatexp.iter().map(|x| *x as i64).collect(),
            cstats: user.e.cstat.to_vec(),
            cstatbuffs: user.e.buffs.to_vec(),
            cstatexp: user.cstatexp.iter().map(|x| *x as i64).collect(),
            pos: user.e.pos,
            vital: user.e.vital.to_vec(),
            deathtimer: user.e.deathtimer,
            indeath: user.e.life.is_spirit(),
            level: user.e.level,
            levelexp: user.levelexp as i64,
            resetcount: user.resetcount,
            pk: user.pk,
        }
    }

    pub fn into_player(self, user: &mut crate::players::Player) {
        user.accid = self.uid;
        user.name = self.name.clone();
        user.addr = self.address.clone();
        user.sprite = self.sprite as u8;
        user.spawn = self.spawn;
        user.itemtimer = self.itemtimer;
        user.vals = self.vals as u64;
        user.data = self.data[..5].try_into().unwrap_or([0; 5]);
        user.access = self.access;
        user.sstats = self.sstats[..SKILL_MAX]
            .try_into()
            .unwrap_or([0; SKILL_MAX]);
        user.sstatbuffs = self.sstatbuffs[..SKILL_MAX]
            .try_into()
            .unwrap_or([0; SKILL_MAX]);
        user.sstatexp = self
            .sstatexp
            .iter()
            .map(|x| *x as u64)
            .collect::<Vec<u64>>()[..SKILL_MAX]
            .try_into()
            .unwrap_or([0; SKILL_MAX]);
        user.e.cstat = self.cstats[..COMBAT_MAX]
            .try_into()
            .unwrap_or([0; COMBAT_MAX]);
        user.e.buffs = self.cstatbuffs[..COMBAT_MAX]
            .try_into()
            .unwrap_or([0; COMBAT_MAX]);
        user.cstatexp = self
            .cstatexp
            .iter()
            .map(|x| *x as u64)
            .collect::<Vec<u64>>()[..COMBAT_MAX]
            .try_into()
            .unwrap_or([0; COMBAT_MAX]);
        user.e.pos = self.pos;
        user.e.vital = self.vital[..VITALS_MAX]
            .try_into()
            .unwrap_or([0; VITALS_MAX]);
        user.e.deathtimer = self.deathtimer;
        user.e.life = match self.indeath {
            true => DeathType::Spirit,
            false => DeathType::Alive,
        };
        user.e.level = self.level;
        user.levelexp = self.levelexp as u64;
        user.resetcount = self.resetcount;
        user.pk = self.pk;
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerLogOut {
    uid: i64,
    itemtimer: MyInstant,
    sstatbuffs: Vec<i16>,
    cstatbuffs: Vec<i16>,
    pos: Position,
    vital: Vec<i32>,
    deathtimer: MyInstant,
    indeath: bool,
    pk: bool,
}

impl PGPlayerLogOut {
    pub fn new(user: &crate::players::Player) -> PGPlayerLogOut {
        PGPlayerLogOut {
            uid: user.accid,
            itemtimer: user.itemtimer,
            sstatbuffs: user.sstatbuffs.to_vec(),
            cstatbuffs: user.e.buffs.to_vec(),
            pos: user.e.pos,
            vital: user.e.vital.to_vec(),
            deathtimer: user.e.deathtimer,
            indeath: user.e.life.is_spirit(),
            pk: user.pk,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerReset {
    uid: i64,
    resetcount: i16,
}

impl PGPlayerReset {
    pub fn new(user: &crate::players::Player) -> PGPlayerReset {
        PGPlayerReset {
            uid: user.accid,
            resetcount: user.resetcount,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerAddress {
    uid: i64,
    address: String,
}

impl PGPlayerAddress {
    pub fn new(user: &crate::players::Player) -> PGPlayerAddress {
        PGPlayerAddress {
            uid: user.accid,
            address: user.addr.clone(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerLevel {
    uid: i64,
    level: i32,
    levelexp: i64,
}

impl PGPlayerLevel {
    pub fn new(user: &crate::players::Player) -> PGPlayerLevel {
        PGPlayerLevel {
            uid: user.accid,
            level: user.e.level,
            levelexp: user.levelexp as i64,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerCombatSkills {
    uid: i64,
    cstats: Vec<i16>,
    cstatexp: Vec<i64>,
}

impl PGPlayerCombatSkills {
    pub fn new(user: &crate::players::Player) -> PGPlayerCombatSkills {
        PGPlayerCombatSkills {
            uid: user.accid,
            cstats: user.e.cstat.to_vec(),
            cstatexp: user.cstatexp.iter().map(|x| *x as i64).collect(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerSkills {
    uid: i64,
    sstats: Vec<i16>,
    sstatexp: Vec<i64>,
}

impl PGPlayerSkills {
    pub fn new(user: &crate::players::Player) -> PGPlayerSkills {
        PGPlayerSkills {
            uid: user.accid,
            sstats: user.sstats.to_vec(),
            sstatexp: user.sstatexp.iter().map(|x| *x as i64).collect(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerData {
    uid: i64,
    data: Vec<i64>,
}

impl PGPlayerData {
    pub fn new(user: &crate::players::Player) -> PGPlayerData {
        PGPlayerData {
            uid: user.accid,
            data: user.data.to_vec(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerPassReset {
    uid: i64,
    passresetcode: Option<String>,
}

impl PGPlayerPassReset {
    pub fn new(user: &crate::players::Player, pass: Option<String>) -> PGPlayerPassReset {
        PGPlayerPassReset {
            uid: user.accid,
            passresetcode: pass,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerSpawn {
    uid: i64,
    spawn: Position,
}

impl PGPlayerSpawn {
    pub fn new(user: &crate::players::Player) -> PGPlayerSpawn {
        PGPlayerSpawn {
            uid: user.accid,
            spawn: user.spawn,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerPos {
    uid: i64,
    pos: Position,
}

impl PGPlayerPos {
    pub fn new(user: &crate::players::Player) -> PGPlayerPos {
        PGPlayerPos {
            uid: user.accid,
            pos: user.e.pos,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[table_name = "players"]
#[primary_key(uid)]
pub struct PGPlayerCurrency {
    uid: i64,
    vals: i64,
}

impl PGPlayerCurrency {
    pub fn new(user: &crate::players::Player) -> PGPlayerCurrency {
        PGPlayerCurrency {
            uid: user.accid,
            vals: user.vals as i64,
        }
    }
}
