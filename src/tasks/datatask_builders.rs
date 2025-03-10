use crate::{
    containers::{GlobalKey, World},
    gametypes::*,
    items::*,
    npcs::*,
    players::*,
    socket::*,
};

pub fn move_packet(
    entity: GlobalKey,
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

pub fn warp_packet(entity: GlobalKey, position: Position) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(position)?;

    Ok(buffer)
}

pub fn dir_packet(entity: GlobalKey, dir: u8) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(dir)?;

    Ok(buffer)
}

pub fn death_packet(entity: GlobalKey, life: DeathType) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(life)?;

    Ok(buffer)
}

pub fn npc_spawn_packet(
    world: &mut World,
    entity: GlobalKey,
    did_spawn: bool,
) -> Result<MByteBuffer> {
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

pub fn player_spawn_packet(
    world: &mut World,
    entity: GlobalKey,
    did_spawn: bool,
) -> Result<MByteBuffer> {
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
    id: GlobalKey,
    position: Position,
    item: Item,
    owner: Option<GlobalKey>,
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
    entity: GlobalKey,
    vital: [i32; VITALS_MAX],
    vitalmax: [i32; VITALS_MAX],
) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(vital)?.write(vitalmax)?;

    Ok(buffer)
}

pub fn damage_packet(
    entity: GlobalKey,
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

pub fn level_packet(entity: GlobalKey, level: i32, levelexp: u64) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(level)?.write(levelexp)?;

    Ok(buffer)
}

pub fn unload_entity_packet(entity: GlobalKey) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?;

    Ok(buffer)
}

pub fn attack_packet(entity: GlobalKey) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?;

    Ok(buffer)
}
