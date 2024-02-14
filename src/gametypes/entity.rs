use bytey::{ByteBufferError, ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};

use crate::{gametypes::*, time_ext::MyInstant};
use std::ops::{Deref, DerefMut};

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct Spawn {
    #[derivative(Default(value = "Position::new(10, 10, MapPosition::new(0,0,0))"))]
    pub pos: Position,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub just_spawned: MyInstant,
}

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct Target {
    pub targettype: EntityType,
    pub targetpos: Position,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub targettimer: MyInstant,
}

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct KillCount {
    pub count: u32,
    #[derivative(Default(value = "MyInstant::now()"))]
    pub killcounttimer: MyInstant,
}

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct Vitals {
    #[derivative(Default(value = "[25, 2, 100]"))]
    pub vital: [i32; VITALS_MAX],
    #[derivative(Default(value = "[25, 2, 100]"))]
    pub vitalmax: [i32; VITALS_MAX],
    #[derivative(Default(value = "[0; VITALS_MAX]"))]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[derivative(Default(value = "[0; VITALS_MAX]"))]
    pub regens: [u32; VITALS_MAX],
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Dir(pub u8);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct AttackTimer(#[derivative(Default(value = "MyInstant::now()"))] pub MyInstant);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct DeathTimer(#[derivative(Default(value = "MyInstant::now()"))] pub MyInstant);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct MoveTimer(#[derivative(Default(value = "MyInstant::now()"))] pub MyInstant);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct Combat(#[derivative(Default(value = "MyInstant::now()"))] pub MyInstant);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Physical {
    pub damage: u32,
    pub defense: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntityData(pub [i64; 10]);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hidden(pub bool);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stunned(pub bool);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attacking(pub bool);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
#[derivative(Default)]
pub struct Level(#[derivative(Default(value = "1"))] pub i32);

#[derive(Derivative, Debug, Clone, PartialEq, Eq)]
pub struct InCombat(pub bool);



//the World ID stored in our own Wrapper for Packet sending etc.
//This will help ensure we dont try to deal with outdated stuff if we use
// the entire ID rather than just its internal ID.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Entity(pub hecs::Entity);

impl Deref for Entity {
    type Target = hecs::Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Entity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/*
pub etype: EntityType,
    pub mode: NpcMode, //Player is always None
impl Entity {
    pub fn get_id(&self) -> usize {
        self.etype.get_id()
    }

    pub fn reset_target(&mut self) {
        self.targettype = EntityType::None;
        self.targetpos = Position::default();
    }
}*/

impl ByteBufferWrite for Entity {
    fn write_to_buffer(&self, buffer: &mut bytey::ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer(buffer)
    }

    fn write_to_buffer_le(&self, buffer: &mut bytey::ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer_le(buffer)
    }

    fn write_to_buffer_be(&self, buffer: &mut bytey::ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer_be(buffer)
    }
}

impl ByteBufferRead for Entity {
    fn read_from_buffer(buffer: &mut bytey::ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(Entity(
            hecs::Entity::from_bits(buffer.read::<u64>()?).ok_or(ByteBufferError::OtherError {
                error: "Bits could nto be converted to hecs Entity. Is your Struct wrong?"
                    .to_owned(),
            })?,
        ))
    }

    fn read_from_buffer_le(buffer: &mut bytey::ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(Entity(
            hecs::Entity::from_bits(buffer.read_le::<u64>()?).ok_or(
                ByteBufferError::OtherError {
                    error: "Bits could nto be converted to hecs Entity. Is your Struct wrong?"
                        .to_owned(),
                },
            )?,
        ))
    }

    fn read_from_buffer_be(buffer: &mut bytey::ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(Entity(
            hecs::Entity::from_bits(buffer.read_be::<u64>()?).ok_or(
                ByteBufferError::OtherError {
                    error: "Bits could nto be converted to hecs Entity. Is your Struct wrong?"
                        .to_owned(),
                },
            )?,
        ))
    }
}
