use crate::{gameloop::*, gametypes::*, items::*, npcs::Npc, players::*};
use bytey::{ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};

//Only 42 of these can be sent per Packet
#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct MovePacket {
    pub id: u64,
    pub position: Position,
    pub warp: bool,
    pub dir: u8,
}

impl MovePacket {
    pub fn npc_task(id: u64, position: Position, warp: bool, dir: u8) -> TaskData {
        TaskData::NpcMove(Self {
            id,
            position,
            warp,
            dir,
        })
    }

    pub fn player_task(id: u64, position: Position, warp: bool, dir: u8) -> TaskData {
        TaskData::PlayerMove(Self {
            id,
            position,
            warp,
            dir,
        })
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
    pub fn npc_task(id: u64, dir: u8) -> TaskData {
        TaskData::NpcDir(Self { id, dir })
    }

    pub fn player_task(id: u64, dir: u8) -> TaskData {
        TaskData::PlayerDir(Self { id, dir })
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
    pub fn npc_task(id: u64, life: DeathType) -> TaskData {
        TaskData::NpcDeath(Self { id, life })
    }

    pub fn player_task(id: u64, life: DeathType) -> TaskData {
        TaskData::PlayerDeath(Self { id, life })
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
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
    pub fn npc_task(npc: &Npc) -> TaskData {
        TaskData::NpcSpawn(Self {
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
        })
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlayerSpawnPacket {
    pub access: UserAccess,
    pub dir: u8,
    pub equip: [Item; EQUIPMENT_TYPE_MAX],
    pub hidden: bool,
    //Player global ID
    pub id: u64,
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct MessagePacket {
    //336 bytes 4 messages per packet
    pub channel: ChatChannel,       //1
    pub head: String,               //74
    pub msg: String,                //256
    pub access: Option<UserAccess>, //5
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct MapItemPacket {
    //3 messages per packet
    pub id: u64, //Items map ID
    pub position: Position,
    pub item: Item,
}
