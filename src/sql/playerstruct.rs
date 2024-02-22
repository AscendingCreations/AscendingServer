use crate::{containers::SALT, gametypes::*, players::*, time_ext::*};
use argon2::{Argon2, PasswordHasher};
use password_hash::SaltString;
use sqlx::FromRow;

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct PlayerWithPassword {
    pub uid: i64,
    pub password: String,
}

#[derive(Debug, FromRow)]
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
        entity: &Entity,
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

        PGPlayer {
            name: world.get_or_panic::<&Account>(entity).username.clone(),
            address: world.get_or_panic::<&Socket>(entity).addr.clone(),
            sprite: world.get_or_panic::<Sprite>(entity).id as i16,
            spawn: world.get_or_panic::<Spawn>(entity).pos,
            itemtimer: world.get_or_panic::<PlayerItemTimer>(entity).itemtimer,
            vals: world.get_or_panic::<Money>(entity).vals as i64,
            data: world.get_or_panic::<EntityData>(entity).0.to_vec(),
            access: world.cloned_get_or_panic::<UserAccess>(entity),
            passresetcode: None,
            pos: world.cloned_get_or_panic::<Position>(entity),
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
            deathtimer: world.get_or_panic::<DeathTimer>(entity).0,
            indeath: world.get_or_panic::<DeathType>(entity).is_spirit(),
            email,
            password: hashed_password,
            username,
            level: world.get_or_panic::<Level>(entity).0,
            levelexp: world.get_or_panic::<Player>(entity).levelexp as i64,
            resetcount: world.get_or_panic::<Player>(entity).resetcount,
            pk: world.get_or_panic::<Player>(entity).pk,
        }
    }
}

#[derive(Debug, PartialEq, Eq, FromRow)]
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
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerWithID {
        PGPlayerWithID {
            uid: world.get_or_panic::<&Account>(entity).id,
            name: world.get_or_panic::<&Account>(entity).username.clone(),
            address: world.get_or_panic::<&Socket>(entity).addr.clone(),
            sprite: world.get_or_panic::<Sprite>(entity).id as i16,
            spawn: world.get_or_panic::<Spawn>(entity).pos,
            itemtimer: world.get_or_panic::<PlayerItemTimer>(entity).itemtimer,
            vals: world.get_or_panic::<Money>(entity).vals as i64,
            data: world.get_or_panic::<EntityData>(entity).0.to_vec(),
            access: world.cloned_get_or_panic::<UserAccess>(entity),
            pos: world.cloned_get_or_panic::<Position>(entity),
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
            deathtimer: world.get_or_panic::<DeathTimer>(entity).0,
            indeath: world.get_or_panic::<DeathType>(entity).is_spirit(),
            level: world.get_or_panic::<Level>(entity).0,
            levelexp: world.get_or_panic::<Player>(entity).levelexp as i64,
            resetcount: world.get_or_panic::<Player>(entity).resetcount,
            pk: world.get_or_panic::<Player>(entity).pk,
        }
    }

    pub fn into_player(self, world: &mut hecs::World, entity: &Entity) {
        world
            .get::<&mut Account>(entity.0)
            .expect("Could not find Account")
            .id = self.uid;
        world
            .get::<&mut Account>(entity.0)
            .expect("Could not find Account")
            .username = self.name.clone();
        world
            .get::<&mut Socket>(entity.0)
            .expect("Could not find Socket")
            .addr = self.address.clone();
        world
            .get::<&mut Sprite>(entity.0)
            .expect("Could not find Sprite")
            .id = self.sprite as u32;
        world
            .get::<&mut Spawn>(entity.0)
            .expect("Could not find Spawn")
            .pos = self.spawn;
        world
            .get::<&mut PlayerItemTimer>(entity.0)
            .expect("Could not find PlayerItemTimer")
            .itemtimer = self.itemtimer;
        world
            .get::<&mut Money>(entity.0)
            .expect("Could not find Money")
            .vals = self.vals as u64;
        world
            .get::<&mut EntityData>(entity.0)
            .expect("Could not find EntityData")
            .0 = self.data[..10].try_into().unwrap_or([0; 10]);
        *world
            .get::<&mut UserAccess>(entity.0)
            .expect("Could not find UserAccess") = self.access;
        *world
            .get::<&mut Position>(entity.0)
            .expect("Could not find Position") = self.pos;
        world
            .get::<&mut Vitals>(entity.0)
            .expect("Could not find Vitals")
            .vital = self.vital[..VITALS_MAX]
            .try_into()
            .unwrap_or([0; VITALS_MAX]);
        world
            .get::<&mut DeathTimer>(entity.0)
            .expect("Could not find DeathTimer")
            .0 = self.deathtimer;
        *world
            .get::<&mut DeathType>(entity.0)
            .expect("Could not find DeathType") = match self.indeath {
            true => DeathType::Spirit,
            false => DeathType::Alive,
        };
        world
            .get::<&mut Level>(entity.0)
            .expect("Could not find Level")
            .0 = self.level;
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .levelexp = self.levelexp as u64;
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .resetcount = self.resetcount;
        world
            .get::<&mut Player>(entity.0)
            .expect("Could not find Player")
            .pk = self.pk;
    }
}

#[derive(Debug, FromRow)]
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
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerLogOut {
        PGPlayerLogOut {
            uid: world.get_or_panic::<&Account>(entity).id,
            itemtimer: world.get_or_panic::<PlayerItemTimer>(entity).itemtimer,
            pos: world.cloned_get_or_panic::<Position>(entity),
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
            deathtimer: world.get_or_panic::<DeathTimer>(entity).0,
            indeath: world.get_or_panic::<DeathType>(entity).is_spirit(),
            pk: world.get_or_panic::<Player>(entity).pk,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerReset {
    uid: i64,
    resetcount: i16,
}

impl PGPlayerReset {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerReset {
        PGPlayerReset {
            uid: world.get_or_panic::<&Account>(entity).id,
            resetcount: world.get_or_panic::<Player>(entity).resetcount,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerAddress {
    uid: i64,
    address: String,
}

impl PGPlayerAddress {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerAddress {
        PGPlayerAddress {
            uid: world.get_or_panic::<&Account>(entity).id,
            address: world.get_or_panic::<&Socket>(entity).addr.clone(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerLevel {
    uid: i64,
    level: i32,
    levelexp: i64,
    vital: Vec<i32>,
}

impl PGPlayerLevel {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerLevel {
        PGPlayerLevel {
            uid: world.get_or_panic::<&Account>(entity).id,
            level: world.get_or_panic::<Level>(entity).0,
            levelexp: world.get_or_panic::<Player>(entity).levelexp as i64,
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerData {
    uid: i64,
    data: Vec<i64>,
}

impl PGPlayerData {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerData {
        PGPlayerData {
            uid: world.get_or_panic::<&Account>(entity).id,
            data: world.get_or_panic::<EntityData>(entity).0.to_vec(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerPassReset {
    uid: i64,
    passresetcode: Option<String>,
}

impl PGPlayerPassReset {
    pub fn new(
        world: &mut hecs::World,
        entity: &Entity,
        pass: Option<String>,
    ) -> PGPlayerPassReset {
        PGPlayerPassReset {
            uid: world.get_or_panic::<&Account>(entity).id,
            passresetcode: pass,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerSpawn {
    uid: i64,
    spawn: Position,
}

impl PGPlayerSpawn {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerSpawn {
        PGPlayerSpawn {
            uid: world.get_or_panic::<&Account>(entity).id,
            spawn: world.get_or_panic::<Spawn>(entity).pos,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerPos {
    uid: i64,
    pos: Position,
}

impl PGPlayerPos {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerPos {
        PGPlayerPos {
            uid: world.get_or_panic::<&Account>(entity).id,
            pos: world.cloned_get_or_panic::<Position>(entity),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerCurrency {
    uid: i64,
    vals: i64,
}

impl PGPlayerCurrency {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> PGPlayerCurrency {
        PGPlayerCurrency {
            uid: world.get_or_panic::<&Account>(entity).id,
            vals: world.get_or_panic::<Money>(entity).vals as i64,
        }
    }
}
