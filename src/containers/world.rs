use std::sync::{Arc, Mutex};

use crate::gametypes::{AscendingError, Result};
use bytey::{ByteBuffer, ByteBufferRead, ByteBufferWrite};
use mmap_bytey::{MByteBuffer, MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use slotmap::{Key, KeyData, SecondaryMap, SlotMap, new_key_type};
use uuid::Uuid;

use super::{HashMap, MapItemEntity, NpcEntity, PlayerEntity};

new_key_type! {
    pub struct GlobalKey;
}

#[derive(Default, Clone, Debug)]
pub enum Entity {
    #[default]
    None,
    Player(Arc<Mutex<PlayerEntity>>),
    Npc(Arc<Mutex<NpcEntity>>),
    MapItem(Arc<Mutex<MapItemEntity>>),
}

#[derive(Default)]
pub struct World {
    pub kinds: SlotMap<GlobalKey, EntityKind>,
    pub entities: SecondaryMap<GlobalKey, Entity>,
    pub account_id: HashMap<Uuid, GlobalKey>,
}

impl World {
    /// Returns a Copied Kind, Errors if doesnt Exist.
    pub fn get_kind(&self, key: GlobalKey) -> Result<EntityKind> {
        self.kinds
            .get(key)
            .copied()
            .ok_or(AscendingError::missing_kind())
    }

    /// Returns a Copied Kind or Default of None if doesnt exist.
    pub fn get_kind_or_default(&self, key: GlobalKey) -> EntityKind {
        self.kinds.get(key).copied().unwrap_or_default()
    }

    /// Returns a Copied Global Key, Errors if doesnt Exist.
    pub fn get_account_id(&self, account_id: &Uuid) -> Option<GlobalKey> {
        self.account_id.get(account_id).copied()
    }

    /// Returns a Cloned Entity. This is Efficient since Entity is Arc.
    /// This also Avoids us holding onto the World.
    pub fn get_entity(&self, key: GlobalKey) -> Result<Entity> {
        self.entities
            .get(key)
            .cloned()
            .ok_or(AscendingError::missing_kind())
    }

    /// Returns a Cloned Entity. This is Efficient since Entity is Arc.
    /// This also Avoids us holding onto the World.
    pub fn get_opt_entity(&self, key: GlobalKey) -> Option<Entity> {
        self.entities.get(key).cloned()
    }

    /// Returns a Cloned Entity or Defaulted Entity::None. This is Efficient since Entity is Arc.
    /// This also Avoids us holding onto the World.
    pub fn get_entity_or_default(&self, key: GlobalKey) -> Entity {
        self.entities.get(key).cloned().unwrap_or_default()
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Deserialize,
    Serialize,
    MByteBufferRead,
    MByteBufferWrite,
)]
// Used to seperate GlobalKey data within Hecs World.
pub enum EntityKind {
    #[default]
    None,
    Player,
    Npc,
    MapItem,
}

impl ByteBufferWrite for GlobalKey {
    fn write_to_bytey_buffer(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.data().as_ffi().write_to_bytey_buffer(buffer)
    }

    fn write_to_bytey_buffer_le(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.data().as_ffi().write_to_bytey_buffer_le(buffer)
    }

    fn write_to_bytey_buffer_be(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        self.data().as_ffi().write_to_bytey_buffer_be(buffer)
    }
}

impl ByteBufferRead for GlobalKey {
    fn read_from_bytey_buffer(buffer: &mut ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(GlobalKey::from(KeyData::from_ffi(buffer.read::<u64>()?)))
    }

    fn read_from_bytey_buffer_le(buffer: &mut ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(GlobalKey::from(KeyData::from_ffi(buffer.read_le::<u64>()?)))
    }

    fn read_from_bytey_buffer_be(buffer: &mut ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(GlobalKey::from(KeyData::from_ffi(buffer.read_be::<u64>()?)))
    }
}

impl MByteBufferWrite for GlobalKey {
    fn write_to_mbuffer(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.data().as_ffi().write_to_mbuffer(buffer)
    }

    fn write_to_mbuffer_le(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.data().as_ffi().write_to_mbuffer_le(buffer)
    }

    fn write_to_mbuffer_be(&self, buffer: &mut MByteBuffer) -> mmap_bytey::Result<()> {
        self.data().as_ffi().write_to_mbuffer_be(buffer)
    }
}

impl MByteBufferRead for GlobalKey {
    fn read_from_mbuffer(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(GlobalKey::from(KeyData::from_ffi(buffer.read::<u64>()?)))
    }

    fn read_from_mbuffer_le(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(GlobalKey::from(KeyData::from_ffi(buffer.read_le::<u64>()?)))
    }

    fn read_from_mbuffer_be(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(GlobalKey::from(KeyData::from_ffi(buffer.read_be::<u64>()?)))
    }
}
