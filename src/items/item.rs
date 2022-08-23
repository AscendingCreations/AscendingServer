use bytey::{ByteBuffer, ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Derivative)]
#[derivative(Default)]
pub struct Item {
    pub num: u32,
    pub val: u16,
    #[derivative(Default(value = "1"))]
    pub level: u8,
    pub data: [i16; 5],
}

impl Item {
    pub fn new(num: u32) -> Self {
        Item {
            num,
            ..Default::default()
        }
    }
}

impl ByteBufferRead for Item {
    fn read_from_buffer(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        let mut item = Self::new(buffer.read::<u32>()?);
        item.val = buffer.read()?;
        item.level = buffer.read()?;

        for i in 0..5 {
            item.data[i] = buffer.read()?;
        }

        Ok(item)
    }

    fn read_from_buffer_le(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        let mut item = Self::new(buffer.read_le::<u32>()?);
        item.val = buffer.read_le()?;
        item.level = buffer.read_le()?;

        for i in 0..5 {
            item.data[i] = buffer.read_le()?;
        }

        Ok(item)
    }

    fn read_from_buffer_be(buffer: &mut ByteBuffer) -> bytey::Result<Self> {
        let mut item = Self::new(buffer.read_be::<u32>()?);
        item.val = buffer.read_be()?;
        item.level = buffer.read_be()?;

        for i in 0..5 {
            item.data[i] = buffer.read_be()?;
        }

        Ok(item)
    }
}

impl ByteBufferWrite for &Item {
    fn write_to_buffer(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write(self.num)?;
        buffer.write(self.val)?;
        buffer.write(self.level)?;
        for i in 0..5 {
            buffer.write(self.data[i])?;
        }
        Ok(())
    }
    fn write_to_buffer_le(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_le(self.num)?;
        buffer.write_le(self.val)?;
        buffer.write_le(self.level)?;
        for i in 0..5 {
            buffer.write_le(self.data[i])?;
        }
        Ok(())
    }
    fn write_to_buffer_be(&self, buffer: &mut ByteBuffer) -> bytey::Result<()> {
        buffer.write_be(self.num)?;
        buffer.write_be(self.val)?;
        buffer.write_be(self.level)?;
        for i in 0..5 {
            buffer.write_be(self.data[i])?;
        }
        Ok(())
    }
}

#[inline]
pub fn val_add_rem(val: &mut u16, add: &mut u16, max: u16) -> u16 {
    let rem = max.saturating_sub(*val);

    if rem > 0 {
        if rem >= *add {
            *val = val.saturating_add(*add);
            *add = 0;
        } else {
            *val = max;
            *add = add.saturating_sub(rem);
        }
    }

    *add
}

//Must always preset add to the max the item contains before calling.
#[inline]
pub fn val_add_amount_rem(val1: &mut u16, val2: &mut u16, add: u16, max: u16) -> u16 {
    let rem = max.saturating_sub(*val1);

    if rem > 0 {
        if rem >= add {
            *val1 = val1.saturating_add(add);
            *val2 = val2.saturating_sub(add);
            return 0;
        } else {
            *val1 = max;
            *val2 = val2.saturating_sub(rem);
            return add.saturating_sub(rem);
        }
    }

    add
}
