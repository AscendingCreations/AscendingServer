use super::{IDIncomming, *};
use crate::{containers::*, gametypes::*};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct IDActor {
    pub storage: Storage,
    pub receiver: mpsc::Receiver<IDIncomming>,
    pub entitys: SlotMap<WorldEntityType>,
}

impl IDActor {
    pub fn new(storage: &Storage, receiver: mpsc::Receiver<IDIncomming>) -> Self {
        IDActor {
            storage: storage.clone(),
            receiver,
            entitys: SlotMap::default(),
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        while let Some(packet) = self.receiver.recv().await {
            match packet {
                IDIncomming::RequestNpcSpawn {
                    spawn_map,
                    mut npc,
                    claim,
                } => {
                    let key = self.entitys.insert(WorldEntityType::Npc);
                    npc.key = key;

                    if let Some(map_sender) = self.storage.map_senders.get(&spawn_map) {
                        if let Err(e) = map_sender
                            .send(crate::maps::MapIncomming::SpawnNpc {
                                npc,
                                claimkey: claim,
                            })
                            .await
                        {
                            log::error!("Error: {e}");
                        } else {
                            continue;
                        }
                    }

                    log::warn!(
                        "NPC could not be sent to map {:?}. Deleting entry",
                        spawn_map
                    );
                    self.entitys.remove(key);
                }
                IDIncomming::RequestPlayerSpawn {
                    spawn_map,
                    mut player,
                } => {
                    let key = self.entitys.insert(WorldEntityType::Player);
                    player.key = key;

                    if let Some(map_sender) = self.storage.map_senders.get(&spawn_map) {
                        if let Err(e) = map_sender
                            .send(crate::maps::MapIncomming::SpawnPlayer { player })
                            .await
                        {
                            log::error!("Error: {e}");
                        } else {
                            continue;
                        }
                    }

                    log::warn!(
                        "Player could not be sent to map {:?}. Deleting entry",
                        spawn_map
                    );
                    self.entitys.remove(key);
                }
                IDIncomming::RequestItemSpawn {
                    spawn_map,
                    mut item,
                    claim,
                } => {
                    let key = self.entitys.insert(WorldEntityType::MapItem);
                    item.key = key;

                    if let Some(map_sender) = self.storage.map_senders.get(&spawn_map) {
                        if let Err(e) = map_sender
                            .send(crate::maps::MapIncomming::SpawnMapItem {
                                item,
                                claimkey: claim,
                            })
                            .await
                        {
                            log::error!("Error: {e}");
                        } else {
                            continue;
                        }
                    }

                    log::warn!(
                        "MapItem could not be sent to map {:?}. Deleting entry",
                        spawn_map
                    );
                    self.entitys.remove(key);
                }
                IDIncomming::RemoveEntity { key } => {
                    self.entitys.remove(key);
                }
            }
        }

        Ok(())
    }
}
