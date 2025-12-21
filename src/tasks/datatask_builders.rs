use crate::{
    containers::{DeathType, Entity, GlobalKey, UserAccess, World},
    gametypes::*,
    items::*,
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

pub fn warp_packet(entity: GlobalKey, position: Position, dir: u8) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(position)?.write(dir)?;

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
    if let Some(Entity::Npc(n_data)) = world.get_opt_entity(entity) {
        let n_data = n_data.try_lock()?;

        let mut buffer = MByteBuffer::new()?;
        buffer
            .write(n_data.movement.dir)?
            .write(entity)?
            .write(n_data.combat.level)?
            .write(n_data.combat.death_type)?
            .write(n_data.mode)?
            .write(n_data.index)?
            .write(n_data.combat.physical.damage)?
            .write(n_data.combat.physical.defense)?
            .write(n_data.movement.pos)?
            .write(n_data.sprite.id)?
            .write(n_data.combat.vitals.vital)?
            .write(n_data.combat.vitals.vitalmax)?
            .write(did_spawn)?;

        Ok(buffer)
    } else {
        Err(AscendingError::missing_entity())
    }
}

pub fn player_spawn_packet(
    world: &mut World,
    entity: GlobalKey,
    did_spawn: bool,
) -> Result<MByteBuffer> {
    if let Some(Entity::Player(p_data)) = world.get_opt_entity(entity) {
        let p_data = p_data.try_lock()?;

        let mut buffer = MByteBuffer::new()?;
        buffer
            .write(&p_data.account.username)?
            .write(p_data.movement.dir)?
            .write(entity)?
            .write(p_data.combat.level)?
            .write(p_data.combat.death_type)?
            .write(p_data.combat.physical.damage)?
            .write(p_data.combat.physical.defense)?
            .write(p_data.movement.pos)?
            .write(p_data.sprite.id)?
            .write(p_data.combat.vitals.vital)?
            .write(p_data.combat.vitals.vitalmax)?
            .write(p_data.user_access)?
            .write(p_data.equipment.clone())? //85
            .write(p_data.general.pk)?
            .write(p_data.general.pvpon)?
            .write(did_spawn)?;

        Ok(buffer)
    } else {
        Err(AscendingError::missing_entity())
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
