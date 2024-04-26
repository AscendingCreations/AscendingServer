use crate::{gametypes::*, items::*, npcs::*, players::*};
use bytey::ByteBuffer;
use hecs::{NoSuchEntity, World};

pub fn move_packet(
    entity: Entity,
    position: Position,
    warp: bool,
    switch: bool,
    dir: u8,
) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(32)?;
    buffer
        .write(entity)?
        .write(position)?
        .write(warp)?
        .write(switch)?
        .write(dir)?;

    Ok(buffer)
}

pub fn warp_packet(entity: Entity, position: Position) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(28)?;
    buffer.write(entity)?.write(position)?;

    Ok(buffer)
}

pub fn dir_packet(entity: Entity, dir: u8) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(9)?;
    buffer.write(entity)?.write(dir)?;

    Ok(buffer)
}

pub fn death_packet(entity: Entity, life: DeathType) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(10)?;
    buffer.write(entity)?.write(life)?;

    Ok(buffer)
}

pub fn npc_spawn_packet(world: &mut World, entity: &Entity, did_spawn: bool) -> Result<ByteBuffer> {
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

    if let Some((dir, hidden, level, life, physical, position, sprite, vitals, mode, npc_index)) =
        query.get()
    {
        let mut buffer = ByteBuffer::with_capacity(77)?;
        buffer
            .write(dir.0)?
            .write(hidden.0)?
            .write(entity)?
            .write(level.0)?
            .write(life)?
            .write(mode)?
            .write(npc_index.0)?
            .write(physical.damage)?
            .write(physical.defense)?
            .write(position)?
            .write(sprite.id)?
            .write(vitals.vital)?
            .write(vitals.vitalmax)?
            .write(did_spawn)?;

        Ok(buffer)
    } else {
        Err(AscendingError::HecNoEntity {
            error: NoSuchEntity,
            backtrace: Box::new(std::backtrace::Backtrace::capture()),
        })
    }
}

pub fn player_spawn_packet(
    world: &mut World,
    entity: &Entity,
    did_spawn: bool,
) -> Result<ByteBuffer> {
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
        life,
        physical,
        position,
        sprite,
        vitals,
        access,
        equipment,
        player,
    )) = query.get()
    {
        let mut buffer = ByteBuffer::with_capacity(account.username.len() + 202)?;
        buffer
            .write(&account.username)?
            .write(dir.0)?
            .write(hidden.0)?
            .write(entity)?
            .write(level.0)?
            .write(life)?
            .write(physical.damage)?
            .write(physical.defense)?
            .write(position)?
            .write(sprite.id)?
            .write(vitals.vital)?
            .write(vitals.vitalmax)?
            .write(access)?
            .write(&equipment.items)? //85
            .write(player.pk)?
            .write(player.pvpon)?
            .write(did_spawn)?;

        Ok(buffer)
    } else {
        Err(AscendingError::HecNoEntity {
            error: NoSuchEntity,
            backtrace: Box::new(std::backtrace::Backtrace::capture()),
        })
    }
}

pub fn message_packet(
    channel: MessageChannel,
    head: String,
    msg: String,
    access: Option<UserAccess>,
) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(head.len() + msg.len() + 5)?;
    buffer
        .write(channel)?
        .write(head)?
        .write(msg)?
        .write(access)?;

    Ok(buffer)
}

pub fn map_item_packet(
    id: Entity,
    position: Position,
    item: Item,
    owner: Option<Entity>,
    did_spawn: bool,
) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(55)?;
    buffer
        .write(id)?
        .write(position)?
        .write(item)?
        .write(owner)?
        .write(did_spawn)?;

    Ok(buffer)
}

pub fn vitals_packet(
    entity: Entity,
    vital: [i32; VITALS_MAX],
    vitalmax: [i32; VITALS_MAX],
) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(32)?;
    buffer.write(entity)?.write(vital)?.write(vitalmax)?;

    Ok(buffer)
}

pub fn damage_packet(entity: Entity, damage: u64) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(16)?;
    buffer.write(entity)?.write(damage)?;

    Ok(buffer)
}

pub fn level_packet(entity: Entity, level: i32, levelexp: u64) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(20)?;
    buffer.write(entity)?.write(level)?.write(levelexp)?;

    Ok(buffer)
}

pub fn unload_entity_packet(entity: Entity) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(8)?;
    buffer.write(entity)?;

    Ok(buffer)
}

pub fn attack_packet(entity: Entity) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::with_capacity(8)?;
    buffer.write(entity)?;

    Ok(buffer)
}
