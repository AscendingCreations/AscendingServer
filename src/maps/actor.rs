use super::{MapBroadCasts, MapIncomming, *};
use crate::{
    containers::*, gametypes::*, maps::GridTile, npcs::Npc, players::Player, time_ext::MyInstant,
};
use bit_op::{bit_u8::*, BitOp};
use educe::Educe;
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};
use sqlx::PgPool;
use std::{
    cell::RefCell,
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
    pub players: HopSlotMap<RefCell<Player>>,
    pub npcs: HopSlotMap<RefCell<Npc>>,
    pub items: HopSlotMap<MapItem>,
    pub claims: HopSlotMap<MapClaims>,
    pub claims_by_position: HashMap<Position, EntityKey>,
}

impl MapActor {
    pub async fn runner(mut self) -> Result<()> {
        /*let mut rng = thread_rng();
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

    pub fn remove_player(&mut self, key: EntityKey) -> Option<Player> {
        self.players.remove(key).map(|p| p.take())
    }

    pub fn remove_npc(&mut self, key: EntityKey) -> Option<Npc> {
        self.npcs.remove(key).map(|n| n.take())
    }

    pub fn remove_item(&mut self, key: EntityKey) -> Option<MapItem> {
        self.items.remove(key)
    }

    pub fn remove_item_from_grid(&mut self, pos: Position) {
        if let Some(grid) = self.move_grids.get_mut(&pos.map) {
            grid[pos.as_tile()].item = None;
        }
    }

    pub fn add_item_to_grid(&mut self, pos: Position, key: EntityKey, num: u32, amount: u16) {
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
