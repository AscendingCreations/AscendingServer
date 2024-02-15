use crate::{gametypes::*, items::*, npcs::Npc, players::*};
use bytey::{ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};

//Only 42 of these can be sent per Packet
#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub struct MovePacket {
    //34
    pub id: u64,
    pub position: Position, //24 bytes
    pub warp: bool,
    pub switch: bool,
    pub dir: u8,
}

impl MovePacket {
    pub fn new(id: u64, position: Position, warp: bool, switch: bool, dir: u8) -> Self {
        Self {
            id,
            position,
            warp,
            switch,
            dir,
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub struct DirPacket {
    pub id: u64,
    pub dir: u8,
}

impl DirPacket {
    pub fn new(id: u64, dir: u8) -> Self {
        Self { id, dir }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub struct DeathPacket {
    pub id: u64,
    pub life: DeathType,
}

impl DeathPacket {
    pub fn new(id: u64, life: DeathType) -> Self {
        Self { id, life }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub struct NpcSpawnPacket {
    pub dir: u8,
    pub hidden: bool,
    //Npc global ID
    pub id: u64,
    pub level: i32,
    pub life: DeathType,
    pub mode: NpcMode,
    //The npc data ID for file loading.
    pub num: u64,
    pub pdamage: u32,
    pub pdefense: u32,
    pub position: Position,
    pub sprite: u32,
    pub vital: [i32; VITALS_MAX],
    pub vitalmax: [i32; VITALS_MAX],
}

impl NpcSpawnPacket {
    pub fn new(npc: &Npc) -> Self {
        Self {
            dir: npc.e.dir,
            hidden: npc.e.hidden,
            id: npc.e.etype.get_id() as u64,
            level: npc.e.level,
            life: npc.e.life,
            mode: npc.e.mode,
            num: npc.num,
            pdamage: npc.e.pdamage,
            pdefense: npc.e.pdefense,
            position: npc.e.pos,
            sprite: npc.sprite,
            vital: npc.e.vital,
            vitalmax: npc.e.vitalmax,
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct PlayerSpawnPacket {
    //Player global ID
    pub id: crate::Entity,
    pub name: String,
    pub access: UserAccess,
    pub dir: u8,
    pub equip: Equipment,
    pub hidden: bool,
    pub level: i32,
    pub life: DeathType,
    pub pdamage: u32,
    pub pdefense: u32,
    pub pk: bool,
    pub position: Position,
    pub pvpon: bool,
    pub sprite: u8,
    pub vital: [i32; VITALS_MAX],
    pub vitalmax: [i32; VITALS_MAX],
}

impl PlayerSpawnPacket {
    pub fn new(world: &mut hecs::World, entity: &Entity) -> Self {
        let data = world.entity(entity.0).expect("Could not get Entity");
        Self {
            name: data.get::<&Account>().expect("Could not find Account").name.clone(),
            dir: data.get::<&Dir>().expect("Could not find Dir").0,
            hidden: data.get::<&Hidden>().expect("Could not find Hidden").0,
            id: *player,
            level: data.get::<&Level>().expect("Could not find Level").0,
            life: *data.get::<&DeathType>().expect("Could not find DeathType"),
            pdamage: data.get::<&Physical>().expect("Could not find Physical").damage,
            pdefense: data.get::<&Physical>().expect("Could not find Physical").defense,
            position: *data.get::<&Position>().expect("Could not find Position"),
            sprite: data.get::<&Sprite>().expect("Could not find Sprite").id as u8,
            vital: data.get::<&Vitals>().expect("Could not find Vitals").vital,
            vitalmax: data.get::<&Vitals>().expect("Could not find Vitals").vitalmax,
            access: *data.get::<&UserAccess>().expect("Could not find UserAccess"),
            equip: *data.get::<&Equipment>().expect("Could not find Equipment"),
            pk: data.get::<&Player>().expect("Could not find Player").pk,
            pvpon: data.get::<&Player>().expect("Could not find Player").pvpon,
        }
    }
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct MessagePacket {
    //336 bytes 4 messages per packet
    pub channel: MessageChannel,    //1
    pub head: String,               //74
    pub msg: String,                //256
    pub access: Option<UserAccess>, //5
}

impl MessagePacket {
    pub fn new(
        channel: MessageChannel,
        head: String,
        msg: String,
        access: Option<UserAccess>,
    ) -> Self {
        Self {
            channel,
            head,
            msg,
            access,
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub struct MapItemPacket {
    //3 messages per packet
    pub id: u64, //Items map ID
    pub position: Position,
    pub item: Item,         //
    pub owner: Option<i64>, //9
}

impl MapItemPacket {
    pub fn new(id: u64, position: Position, item: Item, owner: Option<i64>) -> Self {
        Self {
            id,
            position,
            item,
            owner,
        }
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    ByteBufferRead,
    ByteBufferWrite,
)]
pub struct VitalsPacket {
    pub id: u64,
    pub vital: [i32; VITALS_MAX],
    pub vitalmax: [i32; VITALS_MAX],
}

impl VitalsPacket {
    pub fn new(id: u64, vital: [i32; VITALS_MAX], vitalmax: [i32; VITALS_MAX]) -> Self {
        Self {
            id,
            vital,
            vitalmax,
        }
    }
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct DamagePacket {
    //16 bytes per packet
    pub id: u64,     //8
    pub damage: u64, //8
}

impl DamagePacket {
    pub fn new(id: u64, damage: u64) -> Self {
        Self { id, damage }
    }
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct LevelPacket {
    //20 bytes
    pub id: u64,       //8
    pub level: i32,    //4
    pub levelexp: u64, //8
}

impl LevelPacket {
    pub fn new(id: u64, level: i32, levelexp: u64) -> Self {
        Self {
            id,
            level,
            levelexp,
        }
    }
}
