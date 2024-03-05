use crate::{gametypes::*, items::Item, time_ext::MyInstant};
use bytey::{ByteBufferRead, ByteBufferWrite};

#[derive(Copy, Clone, PartialEq, Eq, Default, ByteBufferRead, ByteBufferWrite)]
pub struct MapItem {
    pub id: u64,
    pub item: Item,
    #[bytey(skip)]
    pub despawn: Option<MyInstant>,
    #[bytey(skip)]
    pub ownertimer: Option<MyInstant>,
    #[bytey(skip)]
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
