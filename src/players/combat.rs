use crate::gametypes::*;
use hecs::World;
use std::cmp;

#[inline]
pub fn damage_player(world: &mut World, entity: &crate::Entity, damage: i32) {
    let mut query = world
        .query_one::<&mut Vitals>(entity.0)
        .expect("damage_player could not find query");

    if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize] =
            player_vital.vital[VitalTypes::Hp as usize].saturating_sub(damage);
    }
}

pub fn get_damage_percentage(damage: u32, hp: (u32, u32)) -> f64 {
    let curhp = cmp::min(hp.0, hp.1);
    let abs_damage = cmp::min(damage, curhp) as f64;
    abs_damage / curhp as f64
}