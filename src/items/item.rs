use bytey::{ByteBufferRead, ByteBufferWrite};
use educe::Educe;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Copy,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Educe,
    ByteBufferWrite,
    ByteBufferRead,
    MByteBufferRead,
    MByteBufferWrite,
)]
#[educe(Default)]
pub struct Item {
    //17 bytes
    pub num: u32,
    pub val: u16,
    #[educe(Default = 1)]
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
