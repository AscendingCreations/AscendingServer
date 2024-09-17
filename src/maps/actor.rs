use super::{MapBroadCasts, MapIncomming, *};
use crate::{
    containers::*,
    gametypes::*,
    identity::ClaimsKey,
    maps::GridTile,
    npcs::Npc,
    players::Player,
    tasks::{DataTaskToken, MapSwitchTasks},
    time_ext::MyInstant,
    GlobalKey, HopSlotMap,
};
use bit_op::{bit_u8::*, BitOp};
use educe::Educe;
use mmap_bytey::MByteBuffer;
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use sqlx::PgPool;
use std::{
    cell::RefCell,
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Read,
};
use std::{
    path::Path,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug)]
pub struct MapActor {
    pub position: MapPosition,
    pub storage: Storage,
    pub tick: MyInstant,
    pub zones: [u64; 5], //contains the NPC spawn Count of each Zone.
    //pub move_grid: [GridTile; MAP_MAX_X * MAP_MAX_Y],
    pub spawnable_item: Vec<SpawnItemData>,
    pub move_grids: IndexMap<MapPosition, [GridTile; MAP_MAX_X * MAP_MAX_Y]>,
    pub broadcast_rx: broadcast::Receiver<MapBroadCasts>,
    pub receiver: mpsc::Receiver<MapIncomming>,
    pub players: IndexMap<GlobalKey, RefCell<Player>>,
    pub npcs: IndexMap<GlobalKey, RefCell<Npc>>,
    pub items: IndexMap<GlobalKey, MapItem>,
    //used for internal processes to pass around for resource locking purposes.
    pub claims: slotmap::HopSlotMap<ClaimsKey, MapClaims>,
    pub claims_by_position: HashMap<Position, ClaimsKey>,
    pub time: GameTime,
    pub packet_cache: IndexMap<DataTaskToken, VecDeque<(u32, MByteBuffer, bool)>>,
    //This keeps track of what Things need sending. So we can leave it loaded and only loop whats needed.
    pub packet_cache_ids: IndexSet<DataTaskToken>,
}

impl MapActor {
    pub fn new(
        position: MapPosition,
        storage: Storage,
        broadcast_rx: broadcast::Receiver<MapBroadCasts>,
        receiver: mpsc::Receiver<MapIncomming>,
    ) -> Self {
        MapActor {
            position,
            storage,
            broadcast_rx,
            receiver,
            tick: MyInstant::now(),
            zones: [0, 0, 0, 0, 0],
            spawnable_item: Vec::new(),
            move_grids: IndexMap::default(),
            players: IndexMap::default(),
            npcs: IndexMap::default(),
            items: IndexMap::default(),
            claims: slotmap::HopSlotMap::default(),
            claims_by_position: HashMap::default(),
            time: GameTime::default(),
            packet_cache: IndexMap::default(),
            packet_cache_ids: IndexSet::default(),
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        /*let mut rng = thread_rng();
        let mut spawnable = Vec::new();
        let mut len = storage.npc_ids.read().await.len();

        loop {
            self.tick = MyInstant::now();

        }*/
        Ok(())
    }

    pub fn update_map_items(&mut self) -> Result<()> {
        let mut to_remove = Vec::new();

        for (key, item) in &self.items {
            if let Some(tmr) = item.despawn {
                if tmr <= self.tick {
                    to_remove.push(key)
                }
            }
        }

        for key in to_remove.into_iter() {
            if let Some(_item) = self.remove_item(key) {
                //DataTaskToken::EntityUnload(e_pos.map)
                //.add_task(storage, unload_entity_packet(*entity)?)
                //.await?;
            }
        }

        Ok(())
    }

    pub fn map_path_blocked(
        &self,
        cur_pos: Position,
        next_pos: Position,
        movedir: u8,
        entity_type: WorldEntityType,
    ) -> bool {
        // Directional blocking might be in the wrong order as it should be.
        // 0 down, 1 right, 2 up, 3 left
        let blocked = self.is_dir_blocked(cur_pos, movedir);

        if !blocked {
            return self.is_blocked_tile(next_pos, entity_type);
        }

        blocked
    }

    pub fn is_dir_blocked(&self, cur_pos: Position, movedir: u8) -> bool {
        // Directional blocking might be in the wrong order as it should be.
        // 0 down, 1 right, 2 up, 3 left
        match movedir {
            0 => {
                if let Some(grid) = self.move_grids.get(&cur_pos.map) {
                    grid[cur_pos.as_tile()].dir_block.get(B0) == 0b00000001
                } else {
                    true
                }
            }
            1 => {
                if let Some(grid) = self.move_grids.get(&cur_pos.map) {
                    grid[cur_pos.as_tile()].dir_block.get(B3) == 0b00001000
                } else {
                    true
                }
            }
            2 => {
                if let Some(grid) = self.move_grids.get(&cur_pos.map) {
                    grid[cur_pos.as_tile()].dir_block.get(B1) == 0b00000010
                } else {
                    true
                }
            }
            _ => {
                if let Some(grid) = self.move_grids.get(&cur_pos.map) {
                    grid[cur_pos.as_tile()].dir_block.get(B2) == 0b00000100
                } else {
                    true
                }
            }
        }
    }

    pub fn is_blocked_tile(&self, pos: Position, entity_type: WorldEntityType) -> bool {
        match self.move_grids.get(&pos.map) {
            Some(grid) => match grid[pos.as_tile()].attr {
                GridAttribute::Walkable => false,
                GridAttribute::Entity => {
                    if entity_type == WorldEntityType::MapItem {
                        false
                    } else {
                        grid[pos.as_tile()].count >= 1
                    }
                }
                GridAttribute::Blocked => true,
                GridAttribute::NpcBlock => entity_type == WorldEntityType::Npc,
            },
            None => true,
        }
    }

    pub fn get_surrounding(&self, include_corners: bool) -> Vec<MapPosition> {
        get_surrounding(self.position, include_corners)
    }

    pub fn add_spawnable_item(&mut self, pos: Position, index: u32, amount: u16, timer_set: u64) {
        self.spawnable_item.push(SpawnItemData {
            index,
            amount,
            pos,
            timer_set,
            ..Default::default()
        });
    }

    pub fn remove_entity_from_grid(&mut self, pos: Position) {
        if let Some(grid) = self.move_grids.get_mut(&pos.map) {
            grid[pos.as_tile()].count = grid[pos.as_tile()].count.saturating_sub(1);

            if grid[pos.as_tile()].count == 0 {
                grid[pos.as_tile()].attr = GridAttribute::Walkable;
            }
        }
    }

    pub fn add_entity_to_grid(&mut self, pos: Position) {
        if let Some(grid) = self.move_grids.get_mut(&pos.map) {
            grid[pos.as_tile()].count = grid[pos.as_tile()].count.saturating_add(1);
            grid[pos.as_tile()].attr = GridAttribute::Entity;
        }
    }

    pub fn add_player(&mut self, player: Player) {
        let key = self.players.insert(RefCell::new(player));

        if let Some(player) = self.players.get_mut(key) {
            player.borrow_mut().key = key;
        }
    }

    pub fn add_npc(&mut self, npc: Npc) {
        let key = self.npcs.insert(RefCell::new(npc));

        if let Some(npc) = self.npcs.get_mut(key) {
            npc.borrow_mut().key = key;
        }
    }

    pub fn remove_player(&mut self, key: GlobalKey) -> Option<Player> {
        self.players.remove(key).map(|p| p.take())
    }

    pub fn remove_npc(&mut self, key: GlobalKey) -> Option<Npc> {
        self.npcs.remove(key).map(|n| n.take())
    }

    pub fn remove_item(&mut self, key: GlobalKey) -> Option<MapItem> {
        self.items.remove(key)
    }

    pub fn remove_item_from_grid(&mut self, pos: Position) {
        if let Some(grid) = self.move_grids.get_mut(&pos.map) {
            grid[pos.as_tile()].item = None;
        }
    }

    pub fn add_item_to_grid(&mut self, pos: Position, key: GlobalKey, num: u32, amount: u16) {
        if let Some(grid) = self.move_grids.get_mut(&pos.map) {
            grid[pos.as_tile()].item = Some((key, num, amount));
        }
    }

    pub fn get_dir_mapid(&self, position: MapPosition, dir: MapPosDir) -> Option<MapPosition> {
        let offset = position.map_offset(dir);
        self.move_grids.get(&offset)?;
        Some(offset)
    }
}
