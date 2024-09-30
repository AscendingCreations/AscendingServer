use crate::{gametypes::*, identity::GlobalKey, npcs::Npc, players::Player};
use bytey::{ByteBufferRead, ByteBufferWrite};
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};

#[derive(
    PartialEq,
    Eq,
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    ByteBufferRead,
    ByteBufferWrite,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum TradeStatus {
    #[default]
    None,
    Accepted,
    Submitted,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    MByteBufferRead,
    MByteBufferWrite,
    ByteBufferRead,
    ByteBufferWrite,
    sqlx::Type,
)]
#[sqlx(type_name = "user_access")]
pub enum UserAccess {
    #[default]
    None,
    Monitor,
    Admin,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    MByteBufferRead,
    MByteBufferWrite,
    Default,
)]
pub enum ChatChannel {
    #[default]
    Map,
    Global,
    Trade,
    Party,
    Guild,
    Whisper,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    speedy::Readable,
    speedy::Writable,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum AIBehavior {
    #[default]
    Friendly, //Never Attack or be attacked
    Agressive,       //Will attack on sight
    Reactive,        //Will attack when attacked
    HelpReactive,    //for npcs that when one gets attacked all in the area target the attacker.
    Healer,          //Will never Attack only heal other npcs
    AgressiveHealer, //Will attack on sight and heal
    ReactiveHealer,  //Will attack when attacked and heal
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum NpcCastType {
    #[default]
    SelfOnly,
    Enemy,  // for Attack spells/bad effects
    Friend, // for healing/revival/good effects
    Ground, // no target just Attack at position
}
impl AIBehavior {
    pub fn is_agressive(&self) -> bool {
        matches!(self, AIBehavior::Agressive | AIBehavior::AgressiveHealer)
    }

    pub fn is_reactive(&self) -> bool {
        matches!(
            self,
            AIBehavior::Reactive | AIBehavior::HelpReactive | AIBehavior::ReactiveHealer
        )
    }

    pub fn is_healer(&self) -> bool {
        matches!(
            self,
            AIBehavior::Healer | AIBehavior::AgressiveHealer | AIBehavior::ReactiveHealer
        )
    }

    pub fn is_friendly(&self) -> bool {
        matches!(self, AIBehavior::Friendly)
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
pub enum WorldEntityType {
    #[default]
    None,
    Player,
    Npc,
    MapItem,
}

//used to pass and to Target GlobalKey's
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
pub enum Target {
    #[default]
    None,
    Player {
        key: GlobalKey,
        uid: i64,
        position: Position,
    }, //ArrID, AccID used for comparison if still same player.
    Npc {
        key: GlobalKey,
        position: Position,
    },
    MapItem {
        key: GlobalKey,
        position: Position,
    },
    Map(Position),
}

impl Target {
    pub fn get_id(&self) -> GlobalKey {
        match self {
            Target::Player {
                key,
                uid: _,
                position: _,
            }
            | Target::Npc { key, position: _ }
            | Target::MapItem { key, position: _ } => *key,
            _ => GlobalKey::default(),
        }
    }

    pub fn get_pos(&self) -> Option<Position> {
        match self {
            Target::Player {
                key: _,
                uid: _,
                position,
            }
            | Target::Npc { key: _, position }
            | Target::MapItem { key: _, position }
            | Target::Map(position) => Some(*position),
            _ => None,
        }
    }

    pub fn update_pos(&mut self, new_position: Position) {
        match self {
            Target::Player {
                key: _,
                uid: _,
                position,
            }
            | Target::Npc { key: _, position }
            | Target::MapItem { key: _, position }
            | Target::Map(position) => *position = new_position,
            _ => {}
        }
    }

    pub fn get_map_pos(&self) -> Option<MapPosition> {
        match self {
            Target::Player {
                key: _,
                uid: _,
                position,
            }
            | Target::Npc { key: _, position }
            | Target::MapItem { key: _, position }
            | Target::Map(position) => Some(position.map),
            _ => None,
        }
    }

    pub fn is_player(&self) -> bool {
        matches!(
            self,
            Target::Player {
                key: _,
                uid: _,
                position: _,
            }
        )
    }

    pub fn is_map(&self) -> bool {
        matches!(self, Target::Map(_))
    }

    pub fn is_npc(&self) -> bool {
        matches!(
            self,
            Target::Npc {
                key: _,
                position: _
            }
        )
    }

    pub fn is_mapitem(&self) -> bool {
        matches!(
            self,
            Target::MapItem {
                key: _,
                position: _
            }
        )
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Target::None)
    }

    pub fn player(key: GlobalKey, uid: i64, position: Position) -> Self {
        Target::Player { key, uid, position }
    }

    pub fn npc(key: GlobalKey, position: Position) -> Self {
        Target::Npc { key, position }
    }

    pub fn map_item(key: GlobalKey, position: Position) -> Self {
        Target::MapItem { key, position }
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
pub enum EntityType {
    #[default]
    None,
    Player(GlobalKey),
    Npc(GlobalKey),
}

#[derive(Debug, Default)]
pub enum Entity<'a> {
    #[default]
    None,
    Player(&'a mut Player),
    Npc(&'a mut Npc),
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    speedy::Readable,
    speedy::Writable,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum ItemTypes {
    #[default]
    None,
    Weapon,
    Accessory,
    Cosmetic,
    Helmet,
    Armor,
    Trouser,
    Boots,
    Consume,
    Tool,
    Blueprint,
    Book,
    Questitem,
    Trap,
    Heavyobject,
    Key,
    Count,
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, MByteBufferRead, MByteBufferWrite,
)]
pub enum EquipmentType {
    Weapon,
    Helmet,
    Chest,
    Pants,
    Accessory,
    Count,
} //5

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum VitalTypes {
    Hp,
    Mp,
    Sp,
    #[default]
    Count,
}

#[derive(
    Debug,
    Copy,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Default,
    speedy::Readable,
    speedy::Writable,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum Weather {
    #[default]
    None,
    Rain,
    Snow,
    Sunny,
    Storm,
    Blizzard,
    Heat,
    Hail,
    SandStorm,
    Windy,
}

#[derive(Copy, Clone)]
pub enum MapLayers {
    Ground,
    Mask,
    Mask2,
    Fringe,
    Fringe2,
    Anim1,
    Anim2,
    Anim3,
    Anim4,
    Anim5,
    Count,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum ToolType {
    #[default]
    None,
    Axe,
    Pick,
    Rod,
    Hoe,
    Scythe,
    Shovel,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum OnlineType {
    None,
    #[default]
    Accepted,
    Online,
}

#[derive(
    Copy,
    Clone,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Default,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum NpcMode {
    None,
    #[default]
    Normal,
    Pet,
    Summon,
    Boss,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    MByteBufferRead,
    MByteBufferWrite,
    sqlx::Type,
)]
#[sqlx(type_name = "log_type")]
pub enum LogType {
    Login,
    Logout,
    Item,
    Warning,
    Error,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    Default,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum IsUsingType {
    #[default]
    None,
    Bank,
    Fishing(i64),
    Crafting(i64),
    Trading(Target),
    Store(i64),
    Other(i64),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SlotSpace {
    NoSpace(u16),
    Completed,
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

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum Death {
    #[default]
    Alive,
    Spirit,
    Dead,
    Spawning,
}

impl Death {
    pub fn is_dead(self) -> bool {
        !matches!(self, Death::Alive)
    }

    pub fn is_spirit(self) -> bool {
        matches!(self, Death::Spirit)
    }

    pub fn is_alive(self) -> bool {
        matches!(self, Death::Alive)
    }

    pub fn is_spawning(self) -> bool {
        matches!(self, Death::Spawning)
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, MByteBufferRead, MByteBufferWrite,
)]
pub enum FtlType {
    Message,
    Error,
    Item,
    Quest,
    Level,
    Money,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MapPos {
    None,
    UpLeft(MapPosition),
    Up(MapPosition),
    UpRight(MapPosition),
    Left(MapPosition),
    Center(MapPosition),
    Right(MapPosition),
    DownLeft(MapPosition),
    Down(MapPosition),
    DownRight(MapPosition),
}

impl MapPos {
    pub fn contains(self, position: MapPosition) -> bool {
        matches!(self,
            MapPos::UpLeft(x)
            | MapPos::Up(x)
            | MapPos::UpRight(x)
            | MapPos::Left(x)
            | MapPos::Center(x)
            | MapPos::Right(x)
            | MapPos::DownLeft(x)
            | MapPos::Down(x)
            | MapPos::DownRight(x)
                if x == position)
    }

    pub fn get(self) -> Option<MapPosition> {
        match self {
            MapPos::UpLeft(x)
            | MapPos::Up(x)
            | MapPos::UpRight(x)
            | MapPos::Left(x)
            | MapPos::Center(x)
            | MapPos::Right(x)
            | MapPos::DownLeft(x)
            | MapPos::Down(x)
            | MapPos::DownRight(x) => Some(x),
            MapPos::None => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MapPosDir {
    None,
    UpLeft,
    Up,
    UpRight,
    Left,
    Center,
    Right,
    DownLeft,
    Down,
    DownRight,
}

impl From<MapPos> for MapPosDir {
    fn from(position: MapPos) -> Self {
        match position {
            MapPos::UpLeft(_) => MapPosDir::UpLeft,
            MapPos::Up(_) => MapPosDir::Up,
            MapPos::UpRight(_) => MapPosDir::UpRight,
            MapPos::Left(_) => MapPosDir::Left,
            MapPos::Center(_) => MapPosDir::Center,
            MapPos::Right(_) => MapPosDir::Right,
            MapPos::DownLeft(_) => MapPosDir::DownLeft,
            MapPos::Down(_) => MapPosDir::Down,
            MapPos::DownRight(_) => MapPosDir::DownRight,
            MapPos::None => MapPosDir::None,
        }
    }
}

impl From<&MapPos> for MapPosDir {
    fn from(position: &MapPos) -> Self {
        match *position {
            MapPos::UpLeft(_) => MapPosDir::UpLeft,
            MapPos::Up(_) => MapPosDir::Up,
            MapPos::UpRight(_) => MapPosDir::UpRight,
            MapPos::Left(_) => MapPosDir::Left,
            MapPos::Center(_) => MapPosDir::Center,
            MapPos::Right(_) => MapPosDir::Right,
            MapPos::DownLeft(_) => MapPosDir::DownLeft,
            MapPos::Down(_) => MapPosDir::Down,
            MapPos::DownRight(_) => MapPosDir::DownRight,
            MapPos::None => MapPosDir::None,
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum MessageChannel {
    #[default]
    Map,
    Global,
    Trade,
    Party,
    Private,
    Guild,
    Help,
    Quest,
    Npc,
}

#[derive(Clone, Debug, PartialEq, Eq, MByteBufferRead, MByteBufferWrite)]
pub enum Command {
    KickPlayer,
    KickPlayerByName(String),
    WarpTo(Position),
    SpawnNpc(i32, Position),
    Trade,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ItemLeftOver {
    None,
    Left(u16),
    Full,
}
