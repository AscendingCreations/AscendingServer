use crate::network::*;
use slotmap::{new_key_type, Key, KeyData};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, MByteBufferRead, MByteBufferWrite)]
pub struct OwnerID {
    key: EntityKey,
    user_id: u64,
}

new_key_type! {
    pub struct EntityKey;
}

impl ByteBufferWrite for EntityKey {
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

impl ByteBufferWrite for &EntityKey {
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

impl ByteBufferRead for EntityKey {
    fn read_from_bytey_buffer(buffer: &mut ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(EntityKey::from(KeyData::from_ffi(buffer.read::<u64>()?)))
    }

    fn read_from_bytey_buffer_le(buffer: &mut ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(EntityKey::from(KeyData::from_ffi(buffer.read_le::<u64>()?)))
    }

    fn read_from_bytey_buffer_be(buffer: &mut ByteBuffer) -> bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(EntityKey::from(KeyData::from_ffi(buffer.read_be::<u64>()?)))
    }
}

impl MByteBufferWrite for EntityKey {
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

impl MByteBufferWrite for &EntityKey {
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

impl MByteBufferRead for EntityKey {
    fn read_from_mbuffer(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(EntityKey::from(KeyData::from_ffi(buffer.read::<u64>()?)))
    }

    fn read_from_mbuffer_le(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(EntityKey::from(KeyData::from_ffi(buffer.read_le::<u64>()?)))
    }

    fn read_from_mbuffer_be(buffer: &mut MByteBuffer) -> mmap_bytey::Result<Self>
    where
        Self: Sized,
    {
        Ok(EntityKey::from(KeyData::from_ffi(buffer.read_be::<u64>()?)))
    }
}
