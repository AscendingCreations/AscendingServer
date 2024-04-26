use crate::{gametypes::*, items::*, npcs::*, players::*};
use bytey::{ByteBufferRead, ByteBufferWrite};
use hecs::{NoSuchEntity, World};
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
    pub entity: Entity,
    pub dir: u8,
    pub hidden: bool,
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
    pub did_spawn: bool,
}

impl NpcSpawnPacket {
    pub fn new(world: &mut World, entity: &Entity, did_spawn: bool) -> Result<Self> {
        let mut query = world.query_one::<(
            &Dir,
            &Hidden,
            &Level,
            &DeathType,
            &Physical,
            &Position,
            &Sprite,
            &Vitals,
            &NpcMode,
            &NpcIndex,
        )>(entity.0)?;

        if let Some((
            dir,
            hidden,
            level,
            &life,
            physical,
            &position,
            sprite,
            vitals,
            &mode,
            npc_index,
        )) = query.get()
        {
            Ok(Self {
                dir: dir.0,
                hidden: hidden.0,
                entity: *entity,
                level: level.0,
                life,
                mode,
                num: npc_index.0,
                pdamage: physical.damage,
                pdefense: physical.defense,
                position,
                sprite: sprite.id,
                vital: vitals.vital,
                vitalmax: vitals.vitalmax,
                did_spawn,
            })
        } else {
            Err(AscendingError::HecNoEntity {
                error: NoSuchEntity,
                backtrace: Box::new(std::backtrace::Backtrace::capture()),
            })
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
    pub did_spawn: bool,
}

impl PlayerSpawnPacket {
    pub fn new(world: &mut World, entity: &Entity, did_spawn: bool) -> Result<Self> {
        let mut query = world.query_one::<(
            &Account,
            &Dir,
            &Hidden,
            &Level,
            &DeathType,
            &Physical,
            &Position,
            &Sprite,
            &Vitals,
            &UserAccess,
            &Equipment,
            &Player,
        )>(entity.0)?;

        if let Some((
            account,
            dir,
            hidden,
            level,
            &life,
            physical,
            &position,
            sprite,
            vitals,
            &access,
            equipment,
            player,
        )) = query.get()
        {
            Ok(Self {
                username: account.username.clone(),
                dir: dir.0,
                hidden: hidden.0,
                entity: *entity,
                level: level.0,
                life,
                pdamage: physical.damage,
                pdefense: physical.defense,
                position,
                sprite: sprite.id as u8,
                vital: vitals.vital,
                vitalmax: vitals.vitalmax,
                access,
                equip: Equipment {
                    items: equipment.items.clone(),
                },
                pk: player.pk,
                pvpon: player.pvpon,
                did_spawn,
            })
        } else {
            Err(AscendingError::HecNoEntity {
                error: NoSuchEntity,
                backtrace: Box::new(std::backtrace::Backtrace::capture()),
            })
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
    pub did_spawn: bool,
}

impl MapItemPacket {
    pub fn new(
        id: Entity,
        position: Position,
        item: Item,
        owner: Option<Entity>,
        did_spawn: bool,
    ) -> Self {
        Self {
            id,
            position,
            item,
            owner,
            did_spawn,
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
