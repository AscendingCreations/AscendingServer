use crate::{
    gametypes::*,
    items::Item,
    //tasks::{map_item_packet, DataTaskToken},
};
use chrono::Duration;
use rand::{thread_rng, Rng};
use std::cmp::min;

use super::{check_surrounding, MapItem};

pub fn create_mapitem(key: EntityKey, index: u32, value: u16, pos: Position) -> MapItem {
    MapItem {
        item: Item {
            num: index,
            val: value,
            ..Default::default()
        },
        despawn: None,
        ownertimer: None,
        ownerid: None,
        pos,
        key,
    }
}
/*
pub async fn spawn_npc(
    world: &GameWorld,
    pos: Position,
    zone: Option<usize>,
    entity: Entity,
) -> Result<()> {
    let lock = world.write().await;
    *lock.get::<&mut Position>(entity.0)? = pos;
    lock.get::<&mut Spawn>(entity.0)?.pos = pos;
    lock.get::<&mut NpcSpawnedZone>(entity.0)?.0 = zone;
    *lock.get::<&mut Death>(entity.0)? = Death::Spawning;

    Ok(())
}

pub fn can_target(
    caster_pos: Position,
    target_pos: Position,
    target_death: Death,
    range: i32,
) -> bool {
    let check = check_surrounding(caster_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    range >= caster_pos.checkdistance(pos) && target_death.is_alive()
}

pub async fn in_dir_attack_zone(
    storage: &GameStore,
    caster_pos: Position,
    target_pos: Position,
    range: i32,
) -> bool {
    let check = check_surrounding(caster_pos.map, target_pos.map, true);
    let pos = target_pos.map_offset(check.into());

    if let Some(dir) = caster_pos.checkdirection(pos) {
        !is_dir_blocked(storage, caster_pos, dir as u8).await
            && range >= caster_pos.checkdistance(pos)
    } else {
        false
    }
}
*/
