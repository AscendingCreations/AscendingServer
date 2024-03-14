use crate::{gametypes::*, items::*, npcs::*, players::*};
use bytey::{ByteBufferRead, ByteBufferWrite};
use hecs::World;
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
pub struct WarpPacket {
    pub entity: Entity,
    pub position: Position,
}

impl WarpPacket {
    pub fn new(entity: Entity, position: Position) -> Self {
        Self { entity, position }
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
    pub sprite: u16,
    pub vital: [i32; VITALS_MAX],
    pub vitalmax: [i32; VITALS_MAX],
}

impl NpcSpawnPacket {
    pub fn new(world: &mut World, entity: &Entity) -> Self {
        Self {
            dir: world.get_or_panic::<Dir>(entity).0,
            hidden: world.get_or_panic::<Hidden>(entity).0,
            entity: *entity,
            level: world.get_or_panic::<Level>(entity).0,
            life: world.cloned_get_or_panic::<DeathType>(entity),
            mode: world.cloned_get_or_panic::<NpcMode>(entity),
            num: world.get_or_panic::<NpcIndex>(entity).0,
            pdamage: world.get_or_panic::<Physical>(entity).damage,
            pdefense: world.get_or_panic::<Physical>(entity).defense,
            position: world.cloned_get_or_panic::<Position>(entity),
            sprite: world.get_or_panic::<Sprite>(entity).id,
            vital: world.get_or_panic::<Vitals>(entity).vital,
            vitalmax: world.get_or_panic::<Vitals>(entity).vitalmax,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ByteBufferRead, ByteBufferWrite)]
pub struct PlayerSpawnPacket {
    //Player global ID
    pub entity: crate::Entity,
    pub username: String,
    pub access: UserAccess,
    pub dir: u8,
    pub equip: Equipment,
    pub hidden: bool,
    pub level: i32,
    pub life: DeathType,
    pub pdamage: u32,
    pub pdefense: u32,
    pub position: Position,
    pub pk: bool,
    pub pvpon: bool,
    pub sprite: u8,
    pub vital: [i32; VITALS_MAX],
    pub vitalmax: [i32; VITALS_MAX],
}

impl PlayerSpawnPacket {
    pub fn new(world: &mut World, entity: &Entity) -> Self {
        Self {
            username: world.get::<&Account>(entity.0).unwrap().username.clone(),
            dir: world.get_or_panic::<Dir>(entity).0,
            hidden: world.get_or_panic::<Hidden>(entity).0,
            entity: *entity,
            level: world.get_or_panic::<Level>(entity).0,
            life: world.cloned_get_or_panic::<DeathType>(entity),
            pdamage: world.get_or_panic::<Physical>(entity).damage,
            pdefense: world.get_or_panic::<Physical>(entity).defense,
            position: world.cloned_get_or_panic::<Position>(entity),
            sprite: world.get_or_panic::<Sprite>(entity).id as u8,
            vital: world.get_or_panic::<Vitals>(entity).vital,
            vitalmax: world.get_or_panic::<Vitals>(entity).vitalmax,
            access: world.cloned_get_or_panic::<UserAccess>(entity),
            equip: Equipment {
                items: world.get::<&Equipment>(entity.0).unwrap().items.clone(),
            },
            pk: world.get_or_panic::<Player>(entity).pk,
            pvpon: world.get_or_panic::<Player>(entity).pvpon,
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
    pub item: Item,            //
    pub owner: Option<Entity>, //9
}

impl MapItemPacket {
    pub fn new(id: Entity, position: Position, item: Item, owner: Option<Entity>) -> Self {
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
pub struct DamagePacket {
    //16 bytes per packet
    pub entity: Entity, //8
    pub damage: u64,    //8
}

impl DamagePacket {
    pub fn new(entity: Entity, damage: u64) -> Self {
        Self { entity, damage }
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
pub struct LevelPacket {
    //20 bytes
    pub entity: Entity, //8
    pub level: i32,     //4
    pub levelexp: u64,  //8
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
