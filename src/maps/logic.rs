use super::{check_surrounding, MapActor};
use crate::gametypes::*;

///use MapItem::new_with() instead. Delete once all are gone.
/*pub fn create_mapitem(index: u32, value: u16, pos: Position) -> MapItem {
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
        key: EntityKey::default(),
    }
}*/

impl MapActor {
    pub fn spawn_npc(&mut self, pos: Position, zone: Option<usize>, key: EntityKey) -> Result<()> {
        if let Some(npc) = self.npcs.get(key) {
            let mut npc = npc.borrow_mut();
            npc.key = key;
            npc.spawn_zone = zone;
            npc.position = pos;
            npc.spawn_pos = pos;
            npc.death_type = Death::Spawning;
        }

        Ok(())
    }

    pub fn in_dir_attack_zone(
        &self,
        caster_pos: Position,
        target_pos: Position,
        range: i32,
    ) -> bool {
        let check = check_surrounding(caster_pos.map, target_pos.map, true);
        let pos = target_pos.map_offset(check.into());

        if let Some(dir) = caster_pos.checkdirection(pos) {
            !self.is_dir_blocked(caster_pos, dir as u8) && range >= caster_pos.checkdistance(pos)
        } else {
            false
        }
    }
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
