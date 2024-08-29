use crate::{
    containers::GameWorld, gametypes::*, players::*, sql::integers::Shifting, time_ext::*,
};
use hecs::NoSuchEntity;
use sqlx::FromRow;
use std::backtrace::Backtrace;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, FromRow)]
pub struct PlayerWithPassword {
    pub uid: i64,
    pub password: String,
}

pub type PlayerQueryMut<'a> = (
    &'a mut Account,
    &'a mut Socket,
    &'a mut Sprite,
    &'a mut Spawn,
    &'a mut PlayerItemTimer,
    &'a mut Money,
    &'a mut EntityData,
    &'a mut UserAccess,
    &'a mut Position,
    &'a mut Vitals,
    &'a mut DeathTimer,
    &'a mut DeathType,
    &'a mut Level,
    &'a mut Player,
);

pub type PlayerQuery<'a> = (
    &'a Account,
    &'a Socket,
    &'a Sprite,
    &'a Spawn,
    &'a PlayerItemTimer,
    &'a Money,
    &'a EntityData,
    &'a UserAccess,
    &'a Position,
    &'a Vitals,
    &'a DeathTimer,
    &'a DeathType,
    &'a Level,
    &'a Player,
);

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
    pub vital_max: Vec<i32>,
    pub deathtimer: MyInstant,
    pub indeath: bool,
    pub level: i32,
    pub levelexp: i64,
    pub resetcount: i16,
    pub pk: bool,
}

impl PGPlayerWithID {
    pub async fn into_player(self, world: &GameWorld, entity: &Entity) -> Result<()> {
        let lock = world.lock().await;
        let mut query = lock.query_one::<PlayerQueryMut>(entity.0)?;

        if let Some((
            account,
            socket,
            sprite,
            spawn,
            itemtimer,
            money,
            entity_data,
            access,
            position,
            vitals,
            death_timer,
            death_type,
            level,
            player,
        )) = query.get()
        {
            account.id = self.uid;
            account.username.clone_from(&self.username);
            socket.addr = Arc::new(self.address);
            sprite.id = self.sprite.shift_signed();
            spawn.pos = self.spawn;
            itemtimer.itemtimer = self.itemtimer;
            money.vals = self.vals.shift_signed();
            entity_data.0 = self.data[..10].try_into().unwrap_or([0; 10]);
            *access = self.access;
            *position = self.pos;
            vitals.vital = self.vital[..VITALS_MAX]
                .try_into()
                .unwrap_or([0; VITALS_MAX]);
            vitals.vitalmax = self.vital_max[..VITALS_MAX]
                .try_into()
                .unwrap_or([0; VITALS_MAX]);
            death_timer.0 = self.deathtimer;
            *death_type = match self.indeath {
                true => DeathType::Spirit,
                false => DeathType::Alive,
            };
            level.0 = self.level;
            player.levelexp = self.levelexp.shift_signed();
            player.resetcount = self.resetcount;
            player.pk = self.pk;
            Ok(())
        } else {
            Err(AscendingError::HecNoEntity {
                error: NoSuchEntity,
                backtrace: Box::new(Backtrace::capture()),
            })
        }
    }
}
