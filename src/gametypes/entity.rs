use crate::{gametypes::*, socket::*, time_ext::MyInstant};
use core::any::type_name;
use educe::Educe;
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
pub struct PlayerTarget(pub Option<GlobalKey>);

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
