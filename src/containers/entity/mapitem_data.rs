use educe::Educe;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};

use crate::{containers::GlobalKey, gametypes::Position, items::Item, time_ext::MyInstant};

#[derive(Debug, Clone, Default)]
pub struct MapItemEntity {
    // General
    pub general: MapItem,

    // Timer
    pub despawn_timer: DespawnTimer,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, MByteBufferRead, MByteBufferWrite)]
pub struct MapItem {
    pub item: Item,
    pub despawn: Option<MyInstant>,
    pub ownertimer: Option<MyInstant>,
    pub ownerid: Option<GlobalKey>,
    pub pos: Position,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct DespawnTimer(#[educe(Default = MyInstant::now())] pub MyInstant);
