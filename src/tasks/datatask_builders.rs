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
    pub dir: u8,
}

impl MovePacket {
    pub fn new(id: u64, position: Position, warp: bool, dir: u8) -> Self {
        Self {
            id,
            position,
            warp,
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
    Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct PlayerSpawnPacket {
    //Player global ID
    pub id: u64,
    pub name: String,
    pub access: UserAccess,
    pub dir: u8,
    pub equip: [Item; EQUIPMENT_TYPE_MAX],
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
    pub fn new(player: &Player) -> Self {
        Self {
            name: player.name.clone(),
            dir: player.e.dir,
            hidden: player.e.hidden,
            id: player.e.etype.get_id() as u64,
            level: player.e.level,
            life: player.e.life,
            pdamage: player.e.pdamage,
            pdefense: player.e.pdefense,
            position: player.e.pos,
            sprite: player.sprite,
            vital: player.e.vital,
            vitalmax: player.e.vitalmax,
            access: player.access,
            equip: player.equip,
            pk: player.pk,
            pvpon: player.pvpon,
        }
    }
}

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite,
)]
pub struct MessagePacket {
    //336 bytes 4 messages per packet
    pub channel: ChatChannel,       //1
    pub head: String,               //74
    pub msg: String,                //256
    pub access: Option<UserAccess>, //5
}

impl MessagePacket {
    pub fn new(
        channel: ChatChannel,
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
    pub item: Item,
    pub owner: Option<u64>,
}

impl MapItemPacket {
    pub fn new(id: u64, position: Position, item: Item, owner: Option<u64>) -> Self {
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
