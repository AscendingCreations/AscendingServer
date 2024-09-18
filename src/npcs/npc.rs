use crate::{
    containers::*, gametypes::*, maps::MapActor, tasks::*, time_ext::MyInstant, GlobalKey,
};
use chrono::Duration;
use educe::Educe;
use std::collections::VecDeque;

use super::NpcData;

#[derive(Debug, Clone, PartialEq, Eq, Educe)]
#[educe(Default)]
pub struct Npc {
    pub index: u64,
    pub key: GlobalKey,
    #[educe(Default = MyInstant::now())]
    pub spawn_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub ai_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub path_timer: MyInstant,
    pub path_tries: usize,
    //when failing to move due to blocks in movement.
    pub path_fails: usize,
    #[educe(Default = false)]
    pub despawn: bool,
    #[educe(Default = false)]
    pub moving: bool,
    #[educe(Default = false)]
    pub retreating: bool,
    #[educe(Default = VecDeque::new())]
    pub npc_moves: VecDeque<(Position, u8)>,
    pub spawn_zone: Option<usize>,
    pub move_pos_overide: Option<Position>,
    #[educe(Default = Position::new(10, 10, MapPosition::new(0,0,0)))]
    pub spawn_pos: Position,
    #[educe(Default  = MyInstant::now())]
    pub just_spawned: MyInstant,
    pub target: Targeting,
    pub kill_count: u32,
    #[educe(Default = MyInstant::now())]
    pub kill_count_timer: MyInstant,
    #[educe(Default = [25, 2, 100])]
    pub vital: [i32; VITALS_MAX],
    #[educe(Default = [25, 2, 100])]
    pub vitalmax: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub regens: [u32; VITALS_MAX],
    pub dir: u8,
    #[educe(Default = MyInstant::now())]
    pub despawn_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub attack_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub death_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub move_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub combat_timer: MyInstant,
    pub damage: u32,
    pub defense: u32,
    pub data: [i64; 10],
    pub hidden: bool,
    pub stunned: bool,
    pub attacking: bool,
    pub in_combat: bool,
    #[educe(Default = 1)]
    pub level: i32,
    pub position: Position,
    pub access: UserAccess,
    pub death: Death,
    pub is_using: IsUsingType,
    pub mode: NpcMode,
    pub sprite: u16,
}

#[inline(always)]
pub fn is_npc_same(from_entity: &GlobalKey, to_entity: &GlobalKey) -> bool {
    from_entity == to_entity
}

impl Npc {
    pub fn new_from(
        map: &MapActor,
        index: u64,
        spawn_pos: Position,
        spawn_zone: Option<usize>,
    ) -> Option<Self> {
        if let Some(npc_data) = map.storage.get_npc(index) {
            Some(Self {
                index,
                key: GlobalKey::default(),
                spawn_timer: map.tick
                    + Duration::try_milliseconds(npc_data.spawn_wait).unwrap_or_default(),
                spawn_zone,
                spawn_pos,
                position: spawn_pos,
                mode: NpcMode::Normal,
                death: Death::Spawning,
                ..Default::default()
            })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn npc_set_move_path(&mut self, path: VecDeque<(Position, u8)>) {
        self.npc_moves = path;
        self.moving = true;
    }

    #[inline(always)]
    pub fn npc_clear_move_path(&mut self) {
        self.npc_moves.clear();
        self.moving = false;
    }

    #[inline(always)]
    pub async fn set_npc_dir(&mut self, map: &MapActor, dir: u8) -> Result<()> {
        if self.dir != dir {
            self.dir = dir;

            /*DataTaskToken::Dir(world.get_or_err::<Position>(entity).await?.map)
            .add_task(storage, dir_packet(*entity, dir)?)
            .await?;*/
        }

        Ok(())
    }

    /* #[inline(always)]
    pub async fn npc_switch_maps(
        world: &GameWorld,
        storage: &GameStore,
        entity: &EntityKey,
        new_pos: Position,
    ) -> Result<Position> {
        let npc_position = world.get_or_err::<Position>(entity).await?;

        if let Some(mapref) = storage.maps.get(&npc_position.map) {
            let mut map = mapref.write().await;
            map.remove_npc(*entity);
            map.remove_entity_from_grid(npc_position);
        } else {
            return Ok(npc_position);
        }

        if let Some(mapref) = storage.maps.get(&new_pos.map) {
            let mut map = mapref.write().await;
            map.add_npc(*entity);
            map.add_entity_to_grid(new_pos);
        } else {
            return Ok(npc_position);
        }

        let lock = world.write().await;
        *lock.get::<&mut Position>(entity.0)? = new_pos;

        Ok(npc_position)
    }*/

    #[inline(always)]
    pub fn npc_swap_pos(&mut self, map: &mut MapActor, pos: Position) -> Position {
        let oldpos = self.position;
        if oldpos != pos {
            self.position = pos;

            map.remove_entity_from_grid(oldpos);
            map.add_entity_to_grid(pos);
        }

        oldpos
    }

    pub async fn npc_getx(&self) -> i32 {
        self.position.x
    }

    pub async fn npc_gety(&self) -> i32 {
        self.position.y
    }

    pub async fn npc_getmap(&self) -> MapPosition {
        self.position.map
    }

    pub async fn npc_gethp(&self) -> i32 {
        self.vital[VitalTypes::Hp as usize]
    }

    pub async fn npc_setx(&mut self, x: i32) {
        self.position.x = x;
    }

    pub async fn npc_sety(&mut self, y: i32) {
        self.position.y = y;
    }

    pub async fn npc_setmap(&mut self, map: MapPosition) {
        self.position.map = map;
    }

    pub async fn npc_sethp(&mut self, hp: i32) {
        self.vital[VitalTypes::Hp as usize] = hp;
    }
}
