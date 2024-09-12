use super::{MapBroadCasts, MapIncomming, *};
use crate::{
    containers::{Config, GameStore, HashSet, IndexMap},
    gametypes::*,
    time_ext::MyInstant,
};
use bit_op::bit_u8::*;
use educe::Educe;
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use sqlx::PgPool;
use std::{
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
    pub npc_count: Arc<AtomicU64>,
    pub player_count: Arc<AtomicU64>,
    pub pgconn: PgPool,
    pub tick: MyInstant,
    pub zones: [u64; 5], //contains the NPC spawn Count of each Zone.
    pub move_grid: [GridTile; MAP_MAX_X * MAP_MAX_Y],
    pub spawnable_item: Vec<SpawnItemData>,
    pub move_grids: IndexMap<MapPosition, [GridTile; MAP_MAX_X * MAP_MAX_Y]>,
    pub senders: IndexMap<MapPosition, mpsc::Sender<MapIncomming>>,
    pub broadcast_tx: broadcast::Sender<MapBroadCasts>,
    pub broadcast_rx: broadcast::Receiver<MapBroadCasts>,
    pub receiver: mpsc::Receiver<MapIncomming>,
    pub config: Arc<Config>,
}

impl MapActor {
    /*
    pub async fn runner(mut self) -> Result<()> {
        let mut rng = thread_rng();
        let mut spawnable = Vec::new();
        let mut len = storage.npc_ids.read().await.len();

        loop {
            self.tick = MyInstant::now();

            let mut count = 0;

            //Spawn NPC's if the max npc's per world is not yet reached.
            if len < MAX_WORLD_NPCS {
                let map = storage
                    .bases
                    .maps
                    .get(position)
                    .ok_or(AscendingError::MapNotFound(*position))?;

                for (id, (max_npcs, zone_npcs)) in self.zones.iter().enumerate() {
                    let data = map_data.read().await;
                    //We want to only allow this many npcs per map to spawn at a time.
                    if count >= NPCS_SPAWNCAP {
                        break;
                    }

                    if !map.zonespawns[id].is_empty() && data.zones[id] < *max_npcs {
                        // Set the Max allowed to spawn by either spawn cap or npc spawn limit.
                        let max_spawnable =
                            min((*max_npcs - data.zones[id]) as usize, NPCS_SPAWNCAP);

                        //Lets Do this for each npc;
                        for npc_id in zone_npcs
                            .iter()
                            .filter(|v| v.is_some())
                            .map(|v| v.unwrap_or_default())
                        {
                            let game_time = storage.time.read().await;
                            let (from, to) = storage
                                .bases
                                .npcs
                                .get(npc_id as usize)
                                .ok_or(AscendingError::NpcNotFound(npc_id))?
                                .spawntime;

                            //Give them a percentage chance to actually spawn
                            //or see if we can spawn them yet within the time frame.
                            if rng.gen_range(0..2) > 0 || !game_time.in_range(from, to) {
                                continue;
                            }

                            //Lets only allow spawning of a set amount each time. keep from over burdening the system.
                            if count >= max_spawnable || len >= MAX_WORLD_NPCS {
                                break;
                            }

                            let mut loop_count = 0;

                            //Only try to find a spot so many times randomly.
                            if !map.zonespawns[id].is_empty() {
                                while loop_count < 10 {
                                    let pos_id = rng.gen_range(0..map.zonespawns[id].len());
                                    let (x, y) = map.zonespawns[id][pos_id];
                                    let spawn = Position::new(x as i32, y as i32, *position);

                                    loop_count += 1;

                                    //Check if the tile is blocked or not.
                                    if !data.is_blocked_tile(spawn, WorldEntityType::Npc) {
                                        //Set NPC as spawnable and to do further checks later.
                                        //Doing this to make the code more readable.
                                        spawnable.push((spawn, id, npc_id));
                                        count = count.saturating_add(1);
                                        len = len.saturating_add(1);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                let mut data = map_data.write().await;
                //Lets Spawn the npcs here;
                for (spawn, zone, npc_id) in spawnable.drain(..) {
                    if let Ok(Some(id)) = storage.add_npc(world, npc_id).await {
                        data.add_npc(id);
                        data.zones[zone] = data.zones[zone].saturating_add(1);
                        spawn_npc(world, spawn, Some(zone), id).await?;
                    }
                }
            }

            let mut add_items = Vec::new();

            for data in map_data.write().await.spawnable_item.iter_mut() {
                let mut storage_mapitem = storage.map_items.write().await;
                if !storage_mapitem.contains_key(&data.pos) {
                    if data.timer <= tick {
                        let map_item = create_mapitem(data.index, data.amount, data.pos);
                        let mut lock = world.write().await;
                        let id = lock.spawn((WorldEntityType::MapItem, map_item));
                        lock.insert(id, (Target::MapItem(Entity(id)), DespawnTimer::default()))?;
                        storage_mapitem.insert(data.pos, Entity(id));
                        DataTaskToken::ItemLoad(data.pos.map)
                            .add_task(
                                storage,
                                map_item_packet(
                                    Entity(id),
                                    map_item.pos,
                                    map_item.item,
                                    map_item.ownerid,
                                    true,
                                )?,
                            )
                            .await?;
                        add_items.push(Entity(id));
                    }
                } else {
                    data.timer = tick
                        + Duration::try_milliseconds(data.timer_set as i64).unwrap_or_default();
                }
            }

            for entity in add_items {
                map_data.write().await.itemids.insert(entity);
            }
        }
        Ok(())
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

    pub fn is_blocked_tile(&self, pos: Position, entity_type: WorldEntityType) -> bool {
        match self.move_grid[pos.as_tile()].attr {
            GridAttribute::Walkable => false,
            GridAttribute::Entity => {
                if entity_type == WorldEntityType::MapItem {
                    false
                } else {
                    self.move_grid[pos.as_tile()].count >= 1
                }
            }
            GridAttribute::Blocked => true,
            GridAttribute::NpcBlock => entity_type == WorldEntityType::Npc,
        }
    }

    pub fn remove_entity_from_grid(&mut self, pos: Position) {
        self.move_grid[pos.as_tile()].count = self.move_grid[pos.as_tile()].count.saturating_sub(1);

        if self.move_grid[pos.as_tile()].count == 0 {
            self.move_grid[pos.as_tile()].attr = GridAttribute::Walkable;
        }
    }

    pub fn add_entity_to_grid(&mut self, pos: Position) {
        self.move_grid[pos.as_tile()].count = self.move_grid[pos.as_tile()].count.saturating_add(1);
        self.move_grid[pos.as_tile()].attr = GridAttribute::Entity;
    }

    pub async fn add_player(&mut self, storage: &GameStore, id: Entity) {
        self.players.insert(id);

        for i in self.get_surrounding(true) {
            if i != self.position {
                match storage.maps.get(&i) {
                    Some(map) => {
                        let mut map_lock = map.write().await;
                        let count = map_lock.players_on_map.saturating_add(1);
                        map_lock.players_on_map = count;
                    }
                    None => continue,
                }
            }
        }

        self.players_on_map = self.players_on_map.saturating_add(1);
    }

    pub fn add_npc(&mut self, id: Entity) {
        self.npcs.insert(id);
    }

    pub async fn remove_player(&mut self, storage: &GameStore, id: Entity) {
        self.players.swap_remove(&id);

        //we set the surrounding maps to have players on them if the player is within 1 map of them.
        for i in self.get_surrounding(true) {
            if i != self.position {
                match storage.maps.get(&i) {
                    Some(map) => {
                        let mut map_lock = map.write().await;
                        let count = map_lock.players_on_map.saturating_sub(1);
                        map_lock.players_on_map = count;
                    }
                    None => continue,
                }
            }
        }

        self.players_on_map = self.players_on_map.saturating_sub(1);
    }

    pub fn remove_npc(&mut self, id: Entity) {
        self.npcs.swap_remove(&id);
    }

    pub fn remove_item(&mut self, id: Entity) {
        /*if !self.items.contains(id) {
            return;
        }*/

        //self.items.remove(id);
        self.itemids.swap_remove(&id);
    }*/
}
