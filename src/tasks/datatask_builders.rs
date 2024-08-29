use crate::{containers::GameWorld, gametypes::*, items::*, npcs::*, players::*, socket::*};
use hecs::NoSuchEntity;

pub fn move_packet(
    entity: Entity,
    position: Position,
    warp: bool,
    switch: bool,
    dir: u8,
) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer
        .write(entity)?
        .write(position)?
        .write(warp)?
        .write(switch)?
        .write(dir)?;

    Ok(buffer)
}

pub fn warp_packet(entity: Entity, position: Position) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(position)?;

    Ok(buffer)
}

pub fn dir_packet(entity: Entity, dir: u8) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(dir)?;

    Ok(buffer)
}

pub fn death_packet(entity: Entity, life: DeathType) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(life)?;

    Ok(buffer)
}

pub async fn npc_spawn_packet(
    world: &GameWorld,
    entity: &Entity,
    did_spawn: bool,
) -> Result<MByteBuffer> {
    let lock = world.lock().await;
    let mut query = lock.query_one::<(
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
        let mut buffer = MByteBuffer::new()?;
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

pub async fn player_spawn_packet(
    world: &GameWorld,
    entity: &Entity,
    did_spawn: bool,
) -> Result<MByteBuffer> {
    let lock: tokio::sync::MutexGuard<'_, hecs::World> = world.lock().await;
    let mut query = lock.query_one::<(
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
        let mut buffer = MByteBuffer::new()?;
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
            .write(equipment)? //85
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
) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
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
) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
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
) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(vital)?.write(vitalmax)?;

    Ok(buffer)
}

pub fn damage_packet(
    entity: Entity,
    damage: u16,
    pos: Position,
    is_damage: bool,
) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer
        .write(entity)?
        .write(damage)?
        .write(pos)?
        .write(is_damage)?;

    Ok(buffer)
}

pub fn level_packet(entity: Entity, level: i32, levelexp: u64) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(level)?.write(levelexp)?;

    Ok(buffer)
}

pub fn unload_entity_packet(entity: Entity) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?;

    Ok(buffer)
}

pub fn attack_packet(entity: Entity) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?;

    Ok(buffer)
}
