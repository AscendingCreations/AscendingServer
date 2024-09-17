use super::{check_surrounding, MapActor};
use crate::{gametypes::*, GlobalKey};
use core::hint::spin_loop;
use mmap_bytey::MByteBuffer;
use rand::{thread_rng, Rng};
use std::{cmp::min, sync::atomic::Ordering};

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
    pub async fn map_spawns(&mut self) -> Result<()> {
        let mut count = 0;
        let mut rng = thread_rng();
        let mut spawnable = Vec::new();

        //Spawn NPC's if the max npc's per world is not yet reached.
        if self.storage.get_npc_count() < self.storage.config.max_npcs {
            let bases = self.storage.bases.clone();

            let map = bases
                .maps
                .get(&self.position)
                .ok_or(AscendingError::MapNotFound(self.position))?;

            for (id, (max_npcs, zone_npcs)) in map.zones.iter().enumerate() {
                //We want to only allow this many npcs per map to spawn at a time.
                if count >= NPCS_SPAWNCAP {
                    break;
                }

                if !map.zonespawns[id].is_empty() && self.zones[id] < *max_npcs {
                    // Set the Max allowed to spawn by either spawn cap or npc spawn limit.
                    let max_spawnable = min((*max_npcs - self.zones[id]) as usize, NPCS_SPAWNCAP);

                    //Lets Do this for each npc;
                    for npc_id in zone_npcs
                        .iter()
                        .filter(|v| v.is_some())
                        .map(|v| v.unwrap_or_default())
                    {
                        let game_time = self.time;
                        let (from, to) = bases
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
                        if count >= max_spawnable
                            || self.storage.npc_count.load(Ordering::SeqCst)
                                >= self.storage.config.max_npcs
                        {
                            break;
                        }

                        let mut loop_count = 0;

                        //Only try to find a spot so many times randomly.
                        if !map.zonespawns[id].is_empty() {
                            while loop_count < 10 {
                                let pos_id = rng.gen_range(0..map.zonespawns[id].len());
                                let (x, y) = map.zonespawns[id][pos_id];
                                let spawn = Position::new(x as i32, y as i32, self.position);

                                loop_count += 1;

                                //Check if the tile is blocked or not.
                                if !self.is_blocked_tile(spawn, WorldEntityType::Npc) {
                                    //Set NPC as spawnable and to do further checks later.
                                    //Doing this to make the code more readable.
                                    spawnable.push((spawn, id, npc_id));
                                    count = count.saturating_add(1);

                                    self.storage.npc_count_add(1);
                                }
                            }
                        }
                    }
                }
            }

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
                if data.timer <= self.tick {
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
                data.timer = self.tick
                    + Duration::try_milliseconds(data.timer_set as i64).unwrap_or_default();
            }
        }

        for entity in add_items {
            map_data.write().await.itemids.insert(entity);
        }
    }

    pub fn spawn_npc(&mut self, pos: Position, zone: Option<usize>, key: GlobalKey) -> Result<()> {
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

    pub async fn send_to(&mut self, key: GlobalKey, mut buf: MByteBuffer) -> Result<()> {
        if let Some(player) = self.players.get(&key) {
            let mut borrow = player.borrow_mut();
            if let Some(socket) = &mut borrow.socket {
                return socket.send(&mut buf).await;
            }
        } else {
            //send to surrounding maps incase they moved?
        }

        Ok(())
    }

    #[inline]
    pub async fn send_to_all(&mut self, buf: MByteBuffer) -> Result<()> {
        let lock = world.read().await;
        for (_entity, (_, socket)) in lock
            .query::<((&WorldEntityType, &OnlineType), &Socket)>()
            .iter()
            .filter(|(_entity, ((worldentitytype, onlinetype), _))| {
                **worldentitytype == WorldEntityType::Player && **onlinetype == OnlineType::Online
            })
        {
            if let Some(client) = storage
                .server
                .read()
                .await
                .clients
                .get(&mio::Token(socket.id))
            {
                client
                    .lock()
                    .await
                    .send(&*storage.poll.read().await, buf.try_clone()?)?;
            }
        }

        Ok(())
    }

    #[inline]
    pub async fn send_to_maps(
        world: &GameWorld,
        storage: &GameStore,
        position: MapPosition,
        buf: MByteBuffer,
        avoidindex: Option<Entity>,
    ) -> Result<()> {
        for m in get_surrounding(position, true) {
            let map = match storage.maps.get(&m) {
                Some(map) => map,
                None => continue,
            }
            .read()
            .await;

            for entity in &map.players {
                if avoidindex.map(|value| value == *entity).unwrap_or(false) {
                    continue;
                }

                let lock = world.read().await;
                let mut query = lock.query_one::<(&OnlineType, &Socket)>(entity.0)?;

                if let Some((status, socket)) = query.get() {
                    if *status == OnlineType::Online {
                        if let Some(client) = storage
                            .server
                            .read()
                            .await
                            .clients
                            .get(&mio::Token(socket.id))
                        {
                            client
                                .lock()
                                .await
                                .send(&*storage.poll.read().await, buf.try_clone()?)?;
                        }
                    }
                }
            }
        }

        Ok(())
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
