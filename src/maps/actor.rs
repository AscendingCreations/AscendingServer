use super::{MapBroadCasts, MapIncomming, *};
use crate::{
    containers::*,
    gametypes::*,
    identity::ClaimsKey,
    maps::GridTile,
    npcs::{Npc, NpcMapInfo},
    players::{Player, PlayerMapInfo},
    tasks::DataTaskToken,
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
    rc::Rc,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::sync::{broadcast, mpsc, Mutex};

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
    pub packet_cache: IndexMap<DataTaskToken, VecDeque<(u32, MByteBuffer, bool)>>,
    //This keeps track of what Things need sending. So we can leave it loaded and only loop whats needed.
    pub packet_cache_ids: IndexSet<DataTaskToken>,
    pub player_switch_processing: IndexSet<GlobalKey>,
}

#[derive(Debug, Default)]
pub struct MapActorStore {
    pub players: IndexMap<GlobalKey, Arc<Mutex<Player>>>,
    pub npcs: IndexMap<GlobalKey, Arc<Mutex<Npc>>>,
    pub items: IndexMap<GlobalKey, MapItem>,
    //used for internal processes to pass around for resource locking purposes.
    pub claims: slotmap::HopSlotMap<ClaimsKey, MapClaims>,
    pub entity_claims_by_position: HashMap<Position, ClaimsKey>,
    pub item_claims_by_position: HashMap<Position, ClaimsKey>,
    pub time: GameTime,
}

impl MapActorStore {
    pub async fn send_to(&mut self, key: GlobalKey, buffer: MByteBuffer) -> Result<()> {
        if let Some(player) = self.players.get_mut(&key) {
            if let Some(socket) = &mut player.lock().await.socket {
                return socket.send(buffer).await;
            }
        } else {
            //send to surrounding maps incase they moved?
        }

        Ok(())
    }

    #[inline]
    pub async fn send_to_all_local(&mut self, buffer: MByteBuffer) -> Result<()> {
        //Send to all of our users first.
        for (_key, player) in &mut self.players {
            if let Some(socket) = &mut player.lock().await.socket {
                return socket.send(buffer.clone()).await;
            }
        }

        Ok(())
    }

    pub fn add_player(&mut self, player: Player, key: GlobalKey) {
        self.players.insert(key, Arc::new(Mutex::new(player)));
    }

    pub fn add_npc(&mut self, npc: Npc, key: GlobalKey) {
        self.npcs.insert(key, Arc::new(Mutex::new(npc)));
    }

    pub fn remove_player(&mut self, key: GlobalKey) -> Option<Player> {
        self.players
            .swap_remove(&key)
            .map(|n| Arc::try_unwrap(n).unwrap().into_inner())
    }

    pub fn remove_npc(&mut self, key: GlobalKey) -> Option<Npc> {
        self.npcs
            .swap_remove(&key)
            .map(|n| Arc::try_unwrap(n).unwrap().into_inner())
    }

    pub fn remove_item(&mut self, key: GlobalKey) -> Option<MapItem> {
        self.items.swap_remove(&key)
    }
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
            zones: [0, 0, 0, 0, 0],
            spawnable_item: Vec::new(),
            move_grids: IndexMap::default(),
            packet_cache: IndexMap::default(),
            packet_cache_ids: IndexSet::default(),
            player_switch_processing: IndexSet::default(),
            tick: MyInstant::now(),
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        let mut store = MapActorStore::default();
        /*let mut rng = thread_rng();
        let mut spawnable = Vec::new();
        let mut len = storage.npc_ids.read().await.len();

        loop {
            self.tick = MyInstant::now();

        }*/
        Ok(())
    }

    pub async fn update_map_items(&self, store: &mut MapActorStore) -> Result<()> {
        let mut to_remove = Vec::new();

        for (key, item) in &store.items {
            if let Some(tmr) = item.despawn {
                if tmr <= self.tick {
                    to_remove.push(*key)
                }
            }
        }

        for key in to_remove.into_iter() {
            if let Some(_item) = store.remove_item(key) {
                //DataTaskToken::EntityUnload(e_pos.map)
                //.add_task(storage, unload_entity_packet(*entity)?)
                //.await?;

                self.storage
                    .id_sender
                    .send(crate::IDIncomming::RemoveEntity { key })
                    .await
                    .unwrap();
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
