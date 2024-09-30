use super::{check_surrounding, MapActor, MapActorStore, MapBroadCasts, MapClaims, MapIncomming};
use crate::{gametypes::*, items::Item, maps::MapItem, npcs::Npc, GlobalKey, IDIncomming};
use chrono::Duration;
use mmap_bytey::MByteBuffer;
use rand::{thread_rng, Rng};
use std::{cmp::min, sync::atomic::Ordering};

//use MapItem::new_with() instead. Delete once all are gone.
//
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
    pub async fn map_spawns(&mut self, store: &mut MapActorStore) -> Result<()> {
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
                        let game_time = store.time;
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
                if let Some(npc) = Npc::new_from(self, npc_id, spawn, Some(zone)) {
                    if !store.entity_claims_by_position.contains_key(&spawn) {
                        continue;
                    }

                    let claim = store.claims.insert(MapClaims::Tile);
                    store.item_claims_by_position.insert(spawn, claim);

                    self.storage
                        .id_sender
                        .send(crate::IDIncomming::RequestNpcSpawn {
                            spawn_map: self.position,
                            npc: Box::new(npc),
                            claim,
                        })
                        .await
                        .unwrap();
                }
            }
        }

        for data in self.spawnable_item.iter_mut() {
            //should never fail.
            let gridtile = self.move_grids.get(&self.position).unwrap();

            if gridtile[data.pos.as_tile()].item.is_none() {
                if data.timer <= self.tick {
                    if !store.item_claims_by_position.contains_key(&data.pos) {
                        continue;
                    }

                    let item = MapItem::new_with(
                        Item::new(data.index, data.amount),
                        data.pos,
                        None,
                        None,
                        None,
                    );
                    let claim = store.claims.insert(MapClaims::ItemSpawn);

                    store.item_claims_by_position.insert(data.pos, claim);
                    self.storage
                        .id_sender
                        .send(IDIncomming::RequestItemSpawn {
                            spawn_map: self.position,
                            item: Box::new(item),
                            claim,
                        })
                        .await
                        .unwrap();

                    /*DataTaskToken::ItemLoad(data.pos.map)
                        .add_task(
                            storage,
                            map_item_packet(
                                GlobalKey(id),
                                map_item.pos,
                                map_item.item,
                                map_item.ownerid,
                                true,
                            )?,
                        )
                        .await?;
                    add_items.push(GlobalKey(id));*/
                }
            } else {
                data.timer = self.tick
                    + Duration::try_milliseconds(data.timer_set as i64).unwrap_or_default();
            }
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

    #[inline]
    pub async fn send_to_all_global(
        &mut self,
        store: &mut MapActorStore,
        buffer: MByteBuffer,
    ) -> Result<()> {
        //Send to all of our users first.
        store.send_to_all_local(buffer.clone()).await?;

        //translate to a clonable value and send as
        let _ = self
            .storage
            .map_broadcast_tx
            .send(MapBroadCasts::SendPacketToAll {
                map_id: self.position,
                buffer,
            });

        Ok(())
    }

    #[inline]
    pub async fn send_to_maps(
        &mut self,
        store: &mut MapActorStore,
        buffer: MByteBuffer,
        avoid: Option<GlobalKey>,
    ) -> Result<()> {
        for (key, player) in &mut store.players {
            if Some(*key) == avoid {
                continue;
            }

            if let Some(socket) = &mut player.socket {
                return socket.send(buffer.clone()).await;
            }
        }

        for m in self.get_surrounding(true) {
            let tx = match self.storage.map_senders.get(&m) {
                Some(map) => map,
                None => continue,
            };

            tx.send(MapIncomming::SendPacketToAll {
                map_id: self.position,
                buffer: buffer.clone(),
                avoid,
            })
            .await
            .unwrap();
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
