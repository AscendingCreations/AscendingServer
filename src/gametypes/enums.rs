use crate::{containers::*, gametypes::*, maps::MapItem};
use bytey::{ByteBufferRead, ByteBufferWrite};
use hecs::World;
use serde::{Deserialize, Serialize};

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
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
    ByteBufferRead,
    ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
)]
// Used to seperate Entity data within Hecs World.
pub enum WorldEntityType {
    #[default]
    None,
    Player,
    Npc,
    MapItem,
    Map,
}

//used to pass and to Target Entity's
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Deserialize,
    Serialize,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub enum EntityType {
    #[default]
    None,
    Player(Entity, i64), //ArrID, AccID used for comparison if still same player.
    Npc(Entity),
    MapItem(Entity),
    Map(Position),
}

impl EntityType {
    pub fn get_id(&self) -> Entity {
        match self {
            EntityType::Player(i, _) | EntityType::Npc(i) | EntityType::MapItem(i) => *i,
            _ => Entity(hecs::Entity::DANGLING),
        }
    }

    pub fn get_pos(&self, world: &mut World, _storage: &Storage) -> Option<Position> {
        match self {
            EntityType::Map(position) => Some(*position),
            EntityType::Player(i, _) => world.get_or_err::<Position>(i).ok(),
            EntityType::Npc(i) => world.get_or_err::<Position>(i).ok(),
            EntityType::MapItem(i) => world
                .get_or_err::<MapItem>(i)
                .map(|mapitem| mapitem.pos)
                .ok(),
            EntityType::None => None,
        }
    }

    pub fn is_player(&self) -> bool {
        matches!(self, EntityType::Player(_, _))
    }

    pub fn is_map(&self) -> bool {
        matches!(self, EntityType::Map(_))
    }

    pub fn is_npc(&self) -> bool {
        matches!(self, EntityType::Npc(_))
    }

    pub fn is_mapitem(&self) -> bool {
        matches!(self, EntityType::MapItem(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, EntityType::None)
    }
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
    ByteBufferRead,
    ByteBufferWrite,
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
    Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ByteBufferRead, ByteBufferWrite,
)]
pub enum EquipmentType {
    Weapon,
    Helmet,
    Chest,
    Pants,
    Belt,
    Boot,
    Accessory1,
    Accessory2,
    Count,
} //8

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Default,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub enum VitalTypes {
    Hp,
    Mp,
    Sp,
    #[default]
    Count,
}

#[derive(
    Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ByteBufferRead, ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
)]
pub enum IsUsingType {
    #[default]
    None,
    Bank,
    Fishing(i64),
    Crafting(i64),
    Trading(Entity),
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

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub enum DeathType {
    #[default]
    Alive,
    Spirit,
    Dead,
    Spawning,
}

impl DeathType {
    pub fn is_dead(self) -> bool {
        !matches!(self, DeathType::Alive)
    }

    pub fn is_spirit(self) -> bool {
        matches!(self, DeathType::Spirit)
    }

    pub fn is_alive(self) -> bool {
        matches!(self, DeathType::Alive)
    }

    pub fn is_spawning(self) -> bool {
        matches!(self, DeathType::Spawning)
    }
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ByteBufferRead, ByteBufferWrite,
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
    ByteBufferRead,
    ByteBufferWrite,
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

#[derive(Clone, Debug, PartialEq, Eq, ByteBufferRead, ByteBufferWrite)]
pub enum Command {
    KickPlayer,
    KickPlayerByName(String),
    WarpTo(Position),
    SpawnNpc(i32, Position),
    Trade,
}
