use crate::{gametypes::*, maps::MapItem, network::*, npcs::*, players::*, GlobalKey};

pub fn move_packet(
    entity: GlobalKey,
    position: Position,
    warp: bool,
    switch: bool,
    dir: Dir,
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

pub fn dir_packet(entity: GlobalKey, dir: Dir) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(dir)?;

    Ok(buffer)
}

pub fn death_packet(entity: GlobalKey, life: Death) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer.write(entity)?.write(life)?;

    Ok(buffer)
}

pub fn npc_spawn_packet(npc: &Npc, did_spawn: bool) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer
        .write(npc.dir)?
        .write(npc.hidden)?
        .write(npc.key)?
        .write(npc.level)?
        .write(npc.death)?
        .write(npc.mode)?
        .write(npc.index)?
        .write(npc.damage)?
        .write(npc.defense)?
        .write(npc.position)?
        .write(npc.sprite)?
        .write(npc.vital)?
        .write(npc.vitalmax)?
        .write(did_spawn)?;

    Ok(buffer)
}

pub fn player_spawn_packet(player: &Player, did_spawn: bool) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer
        .write(&player.username)?
        .write(player.dir)?
        .write(player.hidden)?
        .write(player.key)?
        .write(player.level)?
        .write(player.death)?
        .write(player.damage)?
        .write(player.defense)?
        .write(player.position)?
        .write(player.sprite)?
        .write(player.vital)?
        .write(player.vitalmax)?
        .write(player.access)?
        .write(&player.equipment)? //85
        .write(player.pk)?
        .write(player.pvpon)?
        .write(did_spawn)?;

    Ok(buffer)
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

pub fn map_item_packet(item: &MapItem, did_spawn: bool) -> Result<MByteBuffer> {
    let mut buffer = MByteBuffer::new()?;
    buffer
        .write(item.key)?
        .write(item.pos)?
        .write(item.item)?
        .write(item.ownerid)?
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
