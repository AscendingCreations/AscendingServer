use crate::{gametypes::*, items::Item, time_ext::MyInstant};
use bytey::{ByteBuffer, ByteBufferRead, ByteBufferWrite};

#[derive(Clone, PartialEq, Eq, Default)]
pub struct MapItem {
    pub id: u64,
    pub item: Item,
    pub despawn: Option<MyInstant>,
    pub ownertimer: Option<MyInstant>,
    pub ownerid: i64,
    pub pos: Position,
}

impl MapItem {
    #[inline(always)]
    pub fn new(num: u32) -> Self {
        let mut item = MapItem::default();
        item.item.num = num;
        item
    }
}

impl ByteBufferRead for MapItem {
    fn read_from_buffer(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(Self {
            id: buffer.read()?,
            item: buffer.read()?,
            pos: buffer.read()?,
            ..Default::default()
        })
    }

    fn read_from_buffer_le(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(Self {
            id: buffer.read_le()?,
            item: buffer.read_le()?,
            pos: buffer.read_le()?,
            ..Default::default()
        })
    }

    fn read_from_buffer_be(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        Ok(Self {
            id: buffer.read_be()?,
            item: buffer.read_be()?,
            pos: buffer.read_be()?,
            ..Default::default()
        })
    }
}

impl ByteBufferWrite for &MapItem {
    fn write_to_buffer(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write(self.id)?;
        buffer.write(&self.item)?;
        buffer.write(&self.pos)?;
        Ok(())
    }
    fn write_to_buffer_le(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_le(self.id)?;
        buffer.write_le(&self.item)?;
        buffer.write_le(&self.pos)?;
        Ok(())
    }
    fn write_to_buffer_be(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_be(self.id)?;
        buffer.write_be(&self.item)?;
        buffer.write_be(&self.pos)?;
        Ok(())
    }
}
