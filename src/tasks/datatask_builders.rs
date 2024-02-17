use crate::{gametypes::*, items::*, npcs::*, players::*};
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
    pub entity: Entity,
    pub position: Position, //24 bytes
    pub warp: bool,
    pub switch: bool,
    pub dir: u8,
}

impl MovePacket {
    pub fn new(entity: Entity, position: Position, warp: bool, switch: bool, dir: u8) -> Self {
        Self {
            entity,
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
    pub entity: Entity,
    pub dir: u8,
}

impl DirPacket {
    pub fn new(entity: Entity, dir: u8) -> Self {
        Self { entity, dir }
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
    pub entity: Entity,
    pub life: DeathType,
}

impl DeathPacket {
    pub fn new(entity: Entity, life: DeathType) -> Self {
        Self { entity, life }
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
    pub entity: Entity,
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
    pub fn new(world: &mut hecs::World, entity: &Entity) -> Self {
        let data = world.entity(entity.0).expect("Could not get Entity");
        Self {
            dir: data.get::<&Dir>().expect("Could not find Dir").0,
            hidden: data.get::<&Hidden>().expect("Could not find Hidden").0,
            entity: *entity,
            level: data.get::<&Level>().expect("Could not find Level").0,
            life: *data.get::<&DeathType>().expect("Could not find DeathType"),
            mode: *data.get::<&NpcMode>().expect("Could not find NpcMode"),
            num: data.get::<&NpcIndex>().expect("Could not find NpcIndex").0,
            pdamage: data.get::<&Physical>().expect("Could not find Physical").damage,
            pdefense: data.get::<&Physical>().expect("Could not find Physical").defense,
            position: *data.get::<&Position>().expect("Could not find Position"),
            sprite: data.get::<&Sprite>().expect("Could not find Sprite").id,
            vital: data.get::<&Vitals>().expect("Could not find Vitals").vital,
            vitalmax: data.get::<&Vitals>().expect("Could not find Vitals").vitalmax,
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct PlayerSpawnPacket {
    //Player global ID
    pub entity: crate::Entity,
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
            entity: *entity,
            level: data.get::<&Level>().expect("Could not find Level").0,
            life: *data.get::<&DeathType>().expect("Could not find DeathType"),
            pdamage: data.get::<&Physical>().expect("Could not find Physical").damage,
            pdefense: data.get::<&Physical>().expect("Could not find Physical").defense,
            position: *data.get::<&Position>().expect("Could not find Position"),
            sprite: data.get::<&Sprite>().expect("Could not find Sprite").id as u8,
            vital: data.get::<&Vitals>().expect("Could not find Vitals").vital,
            vitalmax: data.get::<&Vitals>().expect("Could not find Vitals").vitalmax,
            access: *data.get::<&UserAccess>().expect("Could not find UserAccess"),
            equip: Equipment {
                items: data.get::<&Equipment>().expect("Could not find Equipment").items.clone(),
            },
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
    pub id: Entity, //Items map ID
    pub position: Position,
    pub item: Item,         //
    pub owner: Option<i64>, //9
}

impl MapItemPacket {
    pub fn new(id: Entity, position: Position, item: Item, owner: Option<i64>) -> Self {
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
    pub entity: Entity,
    pub vital: [i32; VITALS_MAX],
    pub vitalmax: [i32; VITALS_MAX],
}

impl VitalsPacket {
    pub fn new(entity: Entity, vital: [i32; VITALS_MAX], vitalmax: [i32; VITALS_MAX]) -> Self {
        Self {
            entity,
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
    pub entity: Entity,     //8
    pub damage: u64, //8
}

impl DamagePacket {
    pub fn new(entity: Entity, damage: u64) -> Self {
        Self { entity, damage }
    }
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct LevelPacket {
    //20 bytes
    pub entity: Entity,       //8
    pub level: i32,    //4
    pub levelexp: u64, //8
}

impl LevelPacket {
    pub fn new(entity: Entity, level: i32, levelexp: u64) -> Self {
        Self {
            entity,
            level,
            levelexp,
        }
    }
}
