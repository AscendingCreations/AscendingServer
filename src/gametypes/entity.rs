use crate::{containers::GameWorld, gametypes::*, socket::*, time_ext::MyInstant};
use core::any::type_name;
use educe::Educe;
use hecs::{EntityRef, MissingComponent, World};
use log::{error, warn};
use serde::{Deserialize, Serialize};
use std::{
    backtrace::Backtrace,
    ops::{Deref, DerefMut},
};

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Spawn {
    #[educe(Default = Position::new(10, 10, MapPosition::new(0,0,0)))]
    pub pos: Position,
    #[educe(Default  = MyInstant::now())]
    pub just_spawned: MyInstant,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Target {
    pub target_type: EntityType,
    pub target_pos: Position,
    #[educe(Default = MyInstant::now())]
    pub target_timer: MyInstant,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct ConnectionLoginTimer(#[educe(Default = MyInstant::now())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct PlayerTarget(pub Option<Entity>);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct KillCount {
    pub count: u32,
    #[educe(Default = MyInstant::now())]
    pub killcounttimer: MyInstant,
}

#[derive(
    Educe,
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    ByteBufferWrite,
    ByteBufferRead,
    MByteBufferWrite,
    MByteBufferRead,
)]
#[educe(Default)]
pub struct Vitals {
    #[educe(Default = [25, 2, 100])]
    pub vital: [i32; VITALS_MAX],
    #[educe(Default = [25, 2, 100])]
    pub vitalmax: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub regens: [u32; VITALS_MAX],
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Dir(pub u8);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct DespawnTimer(#[educe(Default = MyInstant::now())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct AttackTimer(#[educe(Default = MyInstant::now())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct DeathTimer(#[educe(Default = MyInstant::now())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct MoveTimer(#[educe(Default = MyInstant::now())] pub MyInstant);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Combat(#[educe(Default = MyInstant::now())] pub MyInstant);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Physical {
    pub damage: u32,
    pub defense: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct EntityData(pub [i64; 10]);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Hidden(pub bool);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Stunned(pub bool);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Attacking(pub bool);

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct Level(#[educe(Default = 1)] pub i32);

#[derive(Educe, Copy, Debug, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct InCombat(#[educe(Default = false)] pub bool);

// The World ID stored in our own Wrapper for Packet sending etc.
// This will help ensure we dont try to deal with outdated stuff if we use
// the entire ID rather than just its internal ID.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Entity(pub hecs::Entity);

impl Deref for Entity {
    type Target = hecs::Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self(hecs::Entity::DANGLING)
    }
}

impl DerefMut for Entity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ByteBufferWrite for Entity {
    fn write_to_buffer(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer(buffer)
    }

    fn write_to_buffer_le(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer_le(buffer)
    }

    fn write_to_buffer_be(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer_be(buffer)
    }
}

impl ByteBufferWrite for &Entity {
    fn write_to_buffer(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer(buffer)
    }

    fn write_to_buffer_le(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer_le(buffer)
    }

    fn write_to_buffer_be(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.0.to_bits().write_to_buffer_be(buffer)
    }
}

impl ByteBufferRead for Entity {
    fn read_from_buffer(buffer: &mut ByteBuffer) -> bytey::Result<Self>
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

    fn read_from_buffer_le(buffer: &mut ByteBuffer) -> bytey::Result<Self>
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

    fn read_from_buffer_be(buffer: &mut ByteBuffer) -> bytey::Result<Self>
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

impl MByteBufferWrite for Entity {
    fn write_to_mbuffer(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.0.to_bits().write_to_mbuffer(buffer)
    }

    fn write_to_mbuffer_le(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.0.to_bits().write_to_mbuffer_le(buffer)
    }

    fn write_to_mbuffer_be(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.0.to_bits().write_to_mbuffer_be(buffer)
    }
}

impl MByteBufferWrite for &Entity {
    fn write_to_mbuffer(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.0.to_bits().write_to_mbuffer(buffer)
    }

    fn write_to_mbuffer_le(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.0.to_bits().write_to_mbuffer_le(buffer)
    }

    fn write_to_mbuffer_be(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.0.to_bits().write_to_mbuffer_be(buffer)
    }
}

impl MByteBufferRead for Entity {
    fn read_from_mbuffer(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(Entity(
            hecs::Entity::from_bits(buffer.read::<u64>()?).ok_or(MByteBufferError::OtherError {
                error: "Bits could nto be converted to hecs Entity. Is your Struct wrong?"
                    .to_owned(),
            })?,
        ))
    }

    fn read_from_mbuffer_le(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(Entity(
            hecs::Entity::from_bits(buffer.read_le::<u64>()?).ok_or(
                MByteBufferError::OtherError {
                    error: "Bits could nto be converted to hecs Entity. Is your Struct wrong?"
                        .to_owned(),
                },
            )?,
        ))
    }

    fn read_from_mbuffer_be(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(Entity(
            hecs::Entity::from_bits(buffer.read_be::<u64>()?).ok_or(
                MByteBufferError::OtherError {
                    error: "Bits could nto be converted to hecs Entity. Is your Struct wrong?"
                        .to_owned(),
                },
            )?,
        ))
    }
}

pub trait WorldExtrasAsync {
    async fn get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Copy + 'static;
    async fn cloned_get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Clone + 'static;
    async fn get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Copy + 'static;
    async fn cloned_get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Clone + 'static;
    async fn get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Copy + 'static;
    async fn cloned_get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Clone + 'static;
    async fn contains(&self, entity: &Entity) -> bool;
}

pub trait WorldExtras {
    fn get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Copy + 'static;
    fn cloned_get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Clone + 'static;
    fn get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Copy + 'static;
    fn cloned_get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Clone + 'static;
    fn get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Copy + 'static;
    fn cloned_get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Clone + 'static;
}

pub trait WorldEntityExtras {
    fn get_or_default<T>(&self) -> T
    where
        T: Default + Send + Sync + Copy + 'static;
    fn cloned_get_or_default<T>(&self) -> T
    where
        T: Default + Send + Sync + Clone + 'static;
    fn get_or_panic<T>(&self) -> T
    where
        T: Send + Sync + Copy + 'static;
    fn cloned_get_or_panic<T>(&self) -> T
    where
        T: Send + Sync + Clone + 'static;
    fn get_or_err<T>(&self) -> Result<T>
    where
        T: Send + Sync + Copy + 'static;
    fn cloned_get_or_err<T>(&self) -> Result<T>
    where
        T: Send + Sync + Clone + 'static;
}

impl WorldEntityExtras for EntityRef<'_> {
    fn get_or_default<T>(&self) -> T
    where
        T: Default + Send + Sync + Copy + 'static,
    {
        self.get::<&T>().map(|t| *t).unwrap_or_default()
    }

    fn cloned_get_or_default<T>(&self) -> T
    where
        T: Default + Send + Sync + Clone + 'static,
    {
        self.get::<&T>().map(|t| (*t).clone()).unwrap_or_default()
    }

    fn get_or_panic<T>(&self) -> T
    where
        T: Send + Sync + Copy + 'static,
    {
        match self.get::<&T>() {
            Some(t) => *t,
            None => {
                error!("Component: {} is missing.", type_name::<T>());
                panic!("Component: {} is missing.", type_name::<T>());
            }
        }
    }

    fn cloned_get_or_panic<T>(&self) -> T
    where
        T: Send + Sync + Clone + 'static,
    {
        match self.get::<&T>() {
            Some(t) => (*t).clone(),
            None => {
                error!("Component: {} is missing.", type_name::<T>());
                panic!("Component: {} is missing.", type_name::<T>());
            }
        }
    }

    fn get_or_err<T>(&self) -> Result<T>
    where
        T: Send + Sync + Copy + 'static,
    {
        match self.get::<&T>().map(|t| *t) {
            Some(t) => Ok(t),
            None => {
                let e = AscendingError::HecsComponent {
                    error: hecs::ComponentError::MissingComponent(MissingComponent::new::<T>()),
                    backtrace: Box::new(Backtrace::capture()),
                };

                warn!("Component Err: {:?}", e);
                Err(e)
            }
        }
    }

    fn cloned_get_or_err<T>(&self) -> Result<T>
    where
        T: Send + Sync + Clone + 'static,
    {
        match self.get::<&T>().map(|t| (*t).clone()) {
            Some(t) => Ok(t),
            None => {
                let e = AscendingError::HecsComponent {
                    error: hecs::ComponentError::MissingComponent(MissingComponent::new::<T>()),
                    backtrace: Box::new(Backtrace::capture()),
                };

                warn!("Component Err: {:?}", e);
                Err(e)
            }
        }
    }
}

impl WorldExtras for World {
    fn get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Copy + 'static,
    {
        self.get::<&T>(entity.0).map(|t| *t).unwrap_or_default()
    }

    fn cloned_get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Clone + 'static,
    {
        self.get::<&T>(entity.0)
            .map(|t| (*t).clone())
            .unwrap_or_default()
    }

    fn get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Copy + 'static,
    {
        match self.get::<&T>(entity.0) {
            Ok(t) => *t,
            Err(e) => {
                error!("Component error: {:?}", e);
                panic!("Component error: {:?}", e);
            }
        }
    }

    fn cloned_get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Clone + 'static,
    {
        match self.get::<&T>(entity.0) {
            Ok(t) => (*t).clone(),
            Err(e) => {
                error!("Component error: {:?}", e);
                panic!("Component error: {:?}", e);
            }
        }
    }

    fn get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Copy + 'static,
    {
        match self.get::<&T>(entity.0).map(|t| *t) {
            Ok(t) => Ok(t),
            Err(e) => {
                warn!("Component Err: {:?}", e);
                Err(AscendingError::HecsComponent {
                    error: e,
                    backtrace: Box::new(Backtrace::capture()),
                })
            }
        }
    }

    fn cloned_get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Clone + 'static,
    {
        match self.get::<&T>(entity.0).map(|t| (*t).clone()) {
            Ok(t) => Ok(t),
            Err(e) => {
                warn!("Component Err: {:?}", e);
                Err(AscendingError::HecsComponent {
                    error: e,
                    backtrace: Box::new(Backtrace::capture()),
                })
            }
        }
    }
}

impl WorldExtrasAsync for GameWorld {
    async fn get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Copy + 'static,
    {
        let lock = self.read().await;
        let data = lock.get::<&T>(entity.0).map(|t| *t).unwrap_or_default();
        data
    }

    async fn cloned_get_or_default<T>(&self, entity: &Entity) -> T
    where
        T: Default + Send + Sync + Clone + 'static,
    {
        let lock = self.read().await;
        let data = lock
            .get::<&T>(entity.0)
            .map(|t| (*t).clone())
            .unwrap_or_default();
        data
    }

    async fn get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Copy + 'static,
    {
        let lock = self.read().await;
        let data = lock.get::<&T>(entity.0);

        let data = match data {
            Ok(t) => *t,
            Err(e) => {
                error!("Component error: {:?}", e);
                panic!("Component error: {:?}", e);
            }
        };

        data
    }

    async fn cloned_get_or_panic<T>(&self, entity: &Entity) -> T
    where
        T: Send + Sync + Clone + 'static,
    {
        let lock = self.read().await;
        let data = lock.get::<&T>(entity.0);
        let data = match data {
            Ok(t) => (*t).clone(),
            Err(e) => {
                error!("Component error: {:?}", e);
                panic!("Component error: {:?}", e);
            }
        };

        data
    }

    async fn get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Copy + 'static,
    {
        let lock = self.read().await;
        let data = lock.get::<&T>(entity.0);
        let data = match data {
            Ok(t) => Ok(*t),
            Err(e) => {
                warn!("Component Err: {:?}", e);
                Err(AscendingError::HecsComponent {
                    error: e,
                    backtrace: Box::new(Backtrace::capture()),
                })
            }
        };
        data
    }

    async fn cloned_get_or_err<T>(&self, entity: &Entity) -> Result<T>
    where
        T: Send + Sync + Clone + 'static,
    {
        let lock = self.read().await;
        let data = lock.get::<&T>(entity.0);
        let data = match data {
            Ok(t) => Ok((*t).clone()),
            Err(e) => {
                warn!("Component Err: {:?}", e);
                Err(AscendingError::HecsComponent {
                    error: e,
                    backtrace: Box::new(Backtrace::capture()),
                })
            }
        };
        data
    }

    async fn contains(&self, entity: &Entity) -> bool {
        let lock = self.read().await;
        let contains = lock.contains(entity.0);
        contains
    }
}
