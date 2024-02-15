use crate::{
    containers::SALT, 
    gametypes::*, 
    sql, 
    time_ext::*,
    players::*,
};
use argon2::{Argon2, PasswordHasher};
use password_hash::SaltString;
use std::convert::TryInto;


#[derive(Queryable, Identifiable, Debug, PartialEq, Eq)]
#[diesel(primary_key(uid))]
#[diesel(table_name = sql::players)]
pub struct PlayerWithPassword {
    pub uid: i64,
    pub password: String,
}

#[derive(Debug, Queryable, Insertable)]
#[diesel(table_name = sql::players)]
pub struct PGPlayer {
    name: String,
    address: String,
    sprite: i16,
    spawn: Position,
    itemtimer: MyInstant,
    vals: i64,
    data: Vec<i64>,
    access: UserAccess,
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
        world: &mut hecs::World,
        player: &Entity,
        username: String,
        email: String,
        password: String,
    ) -> PGPlayer {
        let argon = Argon2::default();
        let hashed_password = if let Ok(salt) = SaltString::encode_b64(SALT) {
            if let Ok(hash) = argon.hash_password(password.as_bytes(), &salt) {
                hash.to_string()
            } else {
                String::from("FailedPasswordHash")
            }
        } else {
            String::from("FailedPasswordHash")
        };

        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayer {
            name: data.get::<&Account>().expect("Could not find Account").name.clone(),
            address: data.get::<&Socket>().expect("Could not find Socket").addr.clone(),
            sprite: data.get::<&Sprite>().expect("Could not find Sprite").id as i16,
            spawn: data.get::<&Spawn>().expect("Could not find Spawn").pos,
            itemtimer: data.get::<&PlayerItemTimer>().expect("Could not find PlayerItemTimer").itemtimer,
            vals: data.get::<&Money>().expect("Could not find Money").vals as i64,
            data: data.get::<&EntityData>().expect("Could not find EntityData").0.to_vec(),
            access: *data.get::<&UserAccess>().expect("Could not find UserAccess").clone(),
            passresetcode: None,
            pos: *data.get::<&Position>().expect("Could not find Position").clone(),
            vital: data.get::<&Vitals>().expect("Could not find Vitals").vital.to_vec(),
            deathtimer: data.get::<&DeathTimer>().expect("Could not find DeathTimer").0,
            indeath: data.get::<&DeathType>().expect("Could not find DeathType").is_spirit(),
            email,
            password: hashed_password,
            username,
            level: data.get::<&Level>().expect("Could not find Level").0,
            levelexp: data.get::<&Player>().expect("Could not find Player").levelexp as i64,
            resetcount: data.get::<&Player>().expect("Could not find Player").resetcount,
            pk: data.get::<&Player>().expect("Could not find Player").pk,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Queryable, Insertable, Identifiable)]
#[diesel(table_name = sql::player_ret)]
#[diesel(primary_key(uid))]
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
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerWithID {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerWithID {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            name: data.get::<&Account>().expect("Could not find Account").name.clone(),
            address: data.get::<&Socket>().expect("Could not find Socket").addr.clone(),
            sprite: data.get::<&Sprite>().expect("Could not find Sprite").id as i16,
            spawn: data.get::<&Spawn>().expect("Could not find Spawn").pos,
            itemtimer: data.get::<&PlayerItemTimer>().expect("Could not find PlayerItemTimer").itemtimer,
            vals: data.get::<&Money>().expect("Could not find Money").vals as i64,
            data: data.get::<&EntityData>().expect("Could not find EntityData").0.to_vec(),
            access: *data.get::<&UserAccess>().expect("Could not find UserAccess").clone(),
            pos: *data.get::<&Position>().expect("Could not find Position").clone(),
            vital: data.get::<&Vitals>().expect("Could not find Vitals").vital.to_vec(),
            deathtimer: data.get::<&DeathTimer>().expect("Could not find DeathTimer").0,
            indeath: data.get::<&DeathType>().expect("Could not find DeathType").is_spirit(),
            level: data.get::<&Level>().expect("Could not find Level").0,
            levelexp: data.get::<&Player>().expect("Could not find Player").levelexp as i64,
            resetcount: data.get::<&Player>().expect("Could not find Player").resetcount,
            pk: data.get::<&Player>().expect("Could not find Player").pk,
        }
    }

    pub fn into_player(self, world: &mut hecs::World, player: &Entity) {
        let data = world.entity(player.0).expect("Could not get Entity");
        if let mut account = data.get::<&mut Account>().expect("Could not find Account") {
            account.id = self.uid;
            account.name = self.name.clone()
        };
        if let mut socket = data.get::<&mut Socket>().expect("Could not find Socket") 
            { socket.addr = self.address.clone() };
        if let mut sprite = data.get::<&mut Sprite>().expect("Could not find Sprite") 
            { sprite.id = self.sprite as u32 };
        if let mut spawn = data.get::<&mut Spawn>().expect("Could not find Spawn") 
            { spawn.pos = self.spawn };
        if let mut itemtimer = data.get::<&mut PlayerItemTimer>().expect("Could not find PlayerItemTimer") { itemtimer.itemtimer = self.itemtimer };
        if let mut money = data.get::<&mut Money>().expect("Could not find Money") 
            { money.vals = self.vals as u64 };
        if let mut data = data.get::<&mut EntityData>().expect("Could not find EntityData") 
            { data.0 = self.data[..10].try_into().unwrap_or([0; 10]) };
        if let mut access = data.get::<&mut UserAccess>().expect("Could not find UserAccess") 
            { *access = self.access };
        if let mut position = data.get::<&mut Position>().expect("Could not find Position") 
            { *position = self.pos };
        if let mut vitals = data.get::<&mut Vitals>().expect("Could not find Vitals") {
            vitals.vital = self.vital[..VITALS_MAX]
                .try_into()
                .unwrap_or([0; VITALS_MAX]);
        };
        if let mut deathtimer = data.get::<&mut DeathTimer>().expect("Could not find DeathTimer") { deathtimer.0 = self.deathtimer };
        if let mut deathtype = data.get::<&mut DeathType>().expect("Could not find DeathType") {
            *deathtype = match self.indeath {
                true => DeathType::Spirit,
                false => DeathType::Alive,
            };
        };
        if let mut level = data.get::<&mut Level>().expect("Could not find Level") 
            { level.0 = self.level };
        if let mut player = data.get::<&mut Player>().expect("Could not find Player") {
            player.levelexp = self.levelexp as u64;
            player.resetcount = self.resetcount;
            player.pk = self.pk;
        };
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerLogOut {
    uid: i64,
    itemtimer: MyInstant,
    pos: Position,
    vital: Vec<i32>,
    deathtimer: MyInstant,
    indeath: bool,
    pk: bool,
}

impl PGPlayerLogOut {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerLogOut {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerLogOut {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            itemtimer: data.get::<&PlayerItemTimer>().expect("Could not find PlayerItemTimer").itemtimer,
            pos: *data.get::<&Position>().expect("Could not find Position").clone(),
            vital: data.get::<&Vitals>().expect("Could not find Vitals").vital.to_vec(),
            deathtimer: data.get::<&DeathTimer>().expect("Could not find DeathTimer").0,
            indeath: data.get::<&DeathType>().expect("Could not find DeathType").is_spirit(),
            pk: data.get::<&Player>().expect("Could not find Player").pk,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerReset {
    uid: i64,
    resetcount: i16,
}

impl PGPlayerReset {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerReset {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerReset {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            resetcount: data.get::<&Player>().expect("Could not find Player").resetcount,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerAddress {
    uid: i64,
    address: String,
}

impl PGPlayerAddress {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerAddress {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerAddress {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            address: data.get::<&Socket>().expect("Could not find Socket").addr.clone(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerLevel {
    uid: i64,
    level: i32,
    levelexp: i64,
    vital: Vec<i32>,
}

impl PGPlayerLevel {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerLevel {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerLevel {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            level: data.get::<&Level>().expect("Could not find Level").0,
            levelexp: data.get::<&Player>().expect("Could not find Player").levelexp as i64,
            vital: data.get::<&Vitals>().expect("Could not find Vitals").vital.to_vec(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerData {
    uid: i64,
    data: Vec<i64>,
}

impl PGPlayerData {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerData {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerData {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            data: data.get::<&EntityData>().expect("Could not find EntityData").0.to_vec(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerPassReset {
    uid: i64,
    passresetcode: Option<String>,
}

impl PGPlayerPassReset {
    pub fn new(world: &mut hecs::World, player: &Entity, pass: Option<String>) -> PGPlayerPassReset {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerPassReset {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            passresetcode: pass,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerSpawn {
    uid: i64,
    spawn: Position,
}

impl PGPlayerSpawn {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerSpawn {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerSpawn {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            spawn: data.get::<&Spawn>().expect("Could not find Spawn").pos,
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerPos {
    uid: i64,
    pos: Position,
}

impl PGPlayerPos {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerPos {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerPos {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            pos: *data.get::<&Position>().expect("Could not find Position").clone(),
        }
    }
}

#[derive(Debug, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = sql::players)]
#[diesel(primary_key(uid))]
pub struct PGPlayerCurrency {
    uid: i64,
    vals: i64,
}

impl PGPlayerCurrency {
    pub fn new(world: &mut hecs::World, player: &Entity) -> PGPlayerCurrency {
        let data = world.entity(player.0).expect("Could not get Entity");
        PGPlayerCurrency {
            uid: data.get::<&Account>().expect("Could not find Account").id,
            vals: data.get::<&Money>().expect("Could not find Money").vals as i64,
        }
    }
}
