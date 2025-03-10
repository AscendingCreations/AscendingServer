use educe::Educe;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{containers::GlobalKey, gametypes::*, items::Item, time_ext::MyInstant};

use super::{CombatData, MovementData};

#[derive(Debug, Clone, Default)]
pub struct PlayerEntity {
    pub account: Account,

    // Connection
    pub socket: Socket,
    pub online_type: OnlineType,
    pub login_handshake: LoginHandShake,
    pub relogin_code: ReloginCode,

    // General Data
    pub sprite: Sprite,
    pub general: Player,
    pub money: Money,
    pub is_using_type: IsUsingType,
    pub user_access: UserAccess,

    // Location
    pub movement: MovementData,

    // Combat
    pub combat: CombatData,

    // Items
    pub inventory: Inventory,
    pub equipment: Equipment,
    pub storage: PlayerStorage,

    pub trade_item: TradeItem,
    pub trade_money: TradeMoney,
    pub trade_status: TradeStatus,
    pub trade_request_entity: TradeRequestEntity,

    // Timer
    pub item_timer: PlayerItemTimer,
    pub map_timer: PlayerMapTimer,
}

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub username: String,
    pub passresetcode: Option<String>,
    pub id: i64,
}

#[derive(Clone, Debug)]
pub struct Socket {
    // IP address
    pub addr: Arc<String>,
    // Socket ID
    pub id: usize,
}

impl Default for Socket {
    fn default() -> Self {
        Self {
            addr: Arc::new(String::new()),
            id: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Educe)]
#[educe(Default)]
pub struct PlayerItemTimer {
    #[educe(Default = MyInstant::now())]
    pub itemtimer: MyInstant,
}

#[derive(Copy, Clone, Debug, Educe)]
#[educe(Default)]
pub struct PlayerMapTimer {
    #[educe(Default = MyInstant::now())]
    pub mapitemtimer: MyInstant,
}

#[derive(
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, MByteBufferRead, MByteBufferWrite,
)]
#[educe(Default)]
pub struct Inventory {
    #[educe(Default = (0..MAX_INV).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, MByteBufferRead, MByteBufferWrite,
)]
#[educe(Default)]
pub struct TradeItem {
    #[educe(Default = (0..MAX_TRADE_SLOT).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TradeMoney {
    pub vals: u64,
}

#[derive(
    PartialEq,
    Eq,
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum TradeStatus {
    #[default]
    None,
    Accepted,
    Submitted,
}

#[derive(Copy, Clone, Debug, Educe)]
#[educe(Default)]
pub struct TradeRequestEntity {
    #[educe(Default = None)]
    pub entity: Option<GlobalKey>,
    #[educe(Default = MyInstant::now())]
    pub requesttimer: MyInstant,
}

#[derive(
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, MByteBufferRead, MByteBufferWrite,
)]
#[educe(Default)]
pub struct PlayerStorage {
    #[educe(Default = (0..MAX_STORAGE).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, MByteBufferRead, MByteBufferWrite,
)]
#[educe(Default)]
pub struct Equipment {
    #[educe(Default = (0..MAX_EQPT).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Sprite {
    pub id: u16,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Money {
    pub vals: u64,
}

#[derive(Educe, Debug, Copy, Clone, PartialEq, Eq)]
#[educe(Default)]
pub struct PlayerConnectionTimer(#[educe(Default = MyInstant::now())] pub MyInstant);

#[derive(Clone, Debug, Default)]
pub struct ReloginCode {
    pub code: String,
}

#[derive(Clone, Debug, Default)]
pub struct LoginHandShake {
    pub handshake: String,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Player {
    pub levelexp: u64,
    pub useditemid: u32,
    pub resetcount: i16,
    pub pvpon: bool,
    pub pk: bool,
    pub movesavecount: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, MByteBufferRead, MByteBufferWrite)]
pub enum IsUsingType {
    #[default]
    None,
    Bank,
    Fishing(i64),
    Crafting(i64),
    Trading(GlobalKey),
    Store(i64),
    Other(i64),
}

impl IsUsingType {
    pub fn inuse(self) -> bool {
        !matches!(self, IsUsingType::None)
    }

    pub fn is_bank(self) -> bool {
        matches!(self, IsUsingType::Bank)
    }

    pub fn is_fishing(self) -> bool {
        matches!(self, IsUsingType::Fishing(_))
    }

    pub fn is_crafting(self) -> bool {
        matches!(self, IsUsingType::Crafting(_))
    }

    pub fn is_trading(self) -> bool {
        matches!(self, IsUsingType::Trading(_))
    }

    pub fn is_instore(self) -> bool {
        matches!(self, IsUsingType::Store(_))
    }

    pub fn is_other(self) -> bool {
        matches!(self, IsUsingType::Other(_))
    }
}
