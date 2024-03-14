use crate::sql::integers::Shifting;
use crate::{containers::SALT, gametypes::*, players::*, time_ext::*};
use argon2::{Argon2, PasswordHasher};
use hecs::World;
use password_hash::SaltString;
use sqlx::FromRow;

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct PlayerWithPassword {
    pub uid: i64,
    pub password: String,
}

#[derive(Debug, FromRow)]
pub struct PGPlayer {
    pub address: String,
    pub sprite: i16,
    pub spawn: Position,
    pub itemtimer: MyInstant,
    pub vals: i64,
    pub data: Vec<i64>,
    pub access: UserAccess,
    pub passresetcode: Option<String>,
    pub pos: Position,
    pub vital: Vec<i32>,
    pub deathtimer: MyInstant,
    pub indeath: bool,
    pub email: String,
    pub password: String,
    pub username: String,
    pub level: i32,
    pub levelexp: i64,
    pub resetcount: i16,
    pub pk: bool,
}

impl PGPlayer {
    pub fn new(
        world: &mut World,
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

        let pos = world.get_or_panic::<Position>(entity);
        let spawn = world.get_or_panic::<Spawn>(entity).pos;

        PGPlayer {
            address: world.get::<&Socket>(entity.0).unwrap().addr.clone(),
            sprite: i16::unshift_signed(&(world.get_or_panic::<Sprite>(entity).id)),
            spawn,
            itemtimer: world.get_or_panic::<PlayerItemTimer>(entity).itemtimer,
            vals: i64::unshift_signed(&world.get_or_panic::<Money>(entity).vals),
            data: world.get_or_panic::<EntityData>(entity).0.to_vec(),
            access: world.cloned_get_or_panic::<UserAccess>(entity),
            passresetcode: None,
            pos,
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
            deathtimer: world.get_or_panic::<DeathTimer>(entity).0,
            indeath: world.get_or_panic::<DeathType>(entity).is_spirit(),
            email,
            password: hashed_password,
            username,
            level: world.get_or_panic::<Level>(entity).0,
            levelexp: i64::unshift_signed(&world.get_or_panic::<Player>(entity).levelexp),
            resetcount: world.get_or_panic::<Player>(entity).resetcount,
            pk: world.get_or_panic::<Player>(entity).pk,
        }
    }
}

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct PGPlayerWithID {
    pub uid: i64,
    pub username: String,
    pub address: String,
    pub sprite: i16,
    pub spawn: Position,
    pub itemtimer: MyInstant,
    pub vals: i64,
    pub data: Vec<i64>,
    pub access: UserAccess,
    pub pos: Position,
    pub vital: Vec<i32>,
    pub deathtimer: MyInstant,
    pub indeath: bool,
    pub level: i32,
    pub levelexp: i64,
    pub resetcount: i16,
    pub pk: bool,
}

impl PGPlayerWithID {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerWithID {
        let account = world.get::<&Account>(entity.0).unwrap();
        let pos = world.get_or_panic::<Position>(entity);
        let spawn = world.get_or_panic::<Spawn>(entity).pos;

        PGPlayerWithID {
            uid: account.id,
            username: account.username.clone(),
            address: world.get::<&Socket>(entity.0).unwrap().addr.clone(),
            sprite: i16::unshift_signed(&world.get_or_panic::<Sprite>(entity).id),
            spawn,
            itemtimer: world.get_or_panic::<PlayerItemTimer>(entity).itemtimer,
            vals: i64::unshift_signed(&world.get_or_panic::<Money>(entity).vals),
            data: world.get_or_panic::<EntityData>(entity).0.to_vec(),
            access: world.cloned_get_or_panic::<UserAccess>(entity),
            pos,
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
            deathtimer: world.get_or_panic::<DeathTimer>(entity).0,
            indeath: world.get_or_panic::<DeathType>(entity).is_spirit(),
            level: world.get_or_panic::<Level>(entity).0,
            levelexp: i64::unshift_signed(&world.get_or_panic::<Player>(entity).levelexp),
            resetcount: world.get_or_panic::<Player>(entity).resetcount,
            pk: world.get_or_panic::<Player>(entity).pk,
        }
    }

    pub fn into_player(self, world: &mut World, entity: &Entity) {
        world
            .get::<&mut Account>(entity.0)
            .expect("Could not find Account")
            .id = self.uid;
        world
            .get::<&mut Account>(entity.0)
            .expect("Could not find Account")
            .username
            .clone_from(&self.username);
        world
            .get::<&mut Socket>(entity.0)
            .expect("Could not find Socket")
            .addr
            .clone_from(&self.address);
        world
            .get::<&mut Sprite>(entity.0)
            .expect("Could not find Sprite")
            .id = self.sprite.shift_signed();
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
            .vals = self.vals.shift_signed();
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
            .levelexp = self.levelexp.shift_signed();
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
    pub uid: i64,
    pub itemtimer: MyInstant,
    pub pos: Position,
    pub vital: Vec<i32>,
    pub deathtimer: MyInstant,
    pub indeath: bool,
    pub pk: bool,
}

impl PGPlayerLogOut {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerLogOut {
        let pos = world.get_or_panic::<Position>(entity);
        PGPlayerLogOut {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            itemtimer: world.get_or_panic::<PlayerItemTimer>(entity).itemtimer,
            pos,
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
            deathtimer: world.get_or_panic::<DeathTimer>(entity).0,
            indeath: world.get_or_panic::<DeathType>(entity).is_spirit(),
            pk: world.get_or_panic::<Player>(entity).pk,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerReset {
    pub uid: i64,
    pub resetcount: i16,
}

impl PGPlayerReset {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerReset {
        PGPlayerReset {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            resetcount: world.get_or_panic::<Player>(entity).resetcount,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerAddress {
    pub uid: i64,
    pub address: String,
}

impl PGPlayerAddress {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerAddress {
        PGPlayerAddress {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            address: world.get::<&Socket>(entity.0).unwrap().addr.clone(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerLevel {
    pub uid: i64,
    pub level: i32,
    pub levelexp: i64,
    pub vital: Vec<i32>,
}

impl PGPlayerLevel {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerLevel {
        PGPlayerLevel {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            level: world.get_or_panic::<Level>(entity).0,
            levelexp: i64::unshift_signed(&world.get_or_panic::<Player>(entity).levelexp),
            vital: world.get_or_panic::<Vitals>(entity).vital.to_vec(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerData {
    pub uid: i64,
    pub data: Vec<i64>,
}

impl PGPlayerData {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerData {
        PGPlayerData {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            data: world.get_or_panic::<EntityData>(entity).0.to_vec(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerPassReset {
    pub uid: i64,
    pub passresetcode: Option<String>,
}

impl PGPlayerPassReset {
    pub fn new(world: &mut World, entity: &Entity, pass: Option<String>) -> PGPlayerPassReset {
        PGPlayerPassReset {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            passresetcode: pass,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerSpawn {
    pub uid: i64,
    pub spawn: Position,
}

impl PGPlayerSpawn {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerSpawn {
        let spawn = world.get_or_panic::<Spawn>(entity).pos;
        PGPlayerSpawn {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            spawn,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerPos {
    pub uid: i64,
    pub pos: Position,
}

impl PGPlayerPos {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerPos {
        let pos = world.cloned_get_or_panic::<Position>(entity);

        PGPlayerPos {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            pos,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct PGPlayerCurrency {
    pub uid: i64,
    pub vals: i64,
}

impl PGPlayerCurrency {
    pub fn new(world: &mut World, entity: &Entity) -> PGPlayerCurrency {
        PGPlayerCurrency {
            uid: world.get::<&Account>(entity.0).unwrap().id,
            vals: i64::unshift_signed(&world.get_or_panic::<Money>(entity).vals),
        }
    }
}
