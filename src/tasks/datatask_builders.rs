use crate::{gametypes::*, items::*, npcs::Npc, players::*, tasks::*};
use bytey::{ByteBufferRead, ByteBufferWrite};
use serde::{Deserialize, Serialize};

//Only 42 of these can be sent per Packet
#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct MovePacket {
    //38
    pub id: u64,
    pub position: Position, //28 bytes
    pub warp: bool,
    pub dir: u8,
}

impl ToBuffer for MovePacket {
    fn to_buffer(&self, buffer: &mut bytey::ByteBuffer) -> Result<()> {
        buffer.write(self.id)?;
        buffer.write(self.position)?;
        buffer.write(self.warp)?;
        buffer.write(self.dir)?;
        Ok(())
    }

    /// This is the Limit at which a Count of these can exist within a packet.
    fn limit(&self) -> u32 {
        36
    }

    fn packet_id(&self) -> u32 {
        ServerPackets::Playermove as u32
    }
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

impl PlayerSpawnPacket {
    pub fn new(player: &Player) -> Self {
        Self {
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

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct MapItemPacket {
    //3 messages per packet
    pub id: u64, //Items map ID
    pub position: Position,
    pub item: Item,
}

impl MapItemPacket {
    pub fn new(id: u64, position: Position, item: Item) -> Self {
        Self { id, position, item }
    }
}
