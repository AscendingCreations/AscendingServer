use crate::{
    gametypes::*,
    maps::*,
    npcs::Npc,
    players::*,
    tasks::{map_item_packet, npc_spawn_packet, player_spawn_packet, DataTaskToken},
    GlobalKey,
};

#[derive(Clone, Debug, Default)]
pub struct PlayerSwitchTasks {
    //So we know we got events back from the maps we sent them before disposing of this data.
    requests: u64,
    npcs: Vec<Npc>,
    players: Vec<Player>,
    items: Vec<MapItem>,
}

impl MapActor {
    pub async fn process_data_lists(&mut self, store: &mut MapActorStore) -> Result<()> {
        let keys: Vec<GlobalKey> = self.player_switch_processing.iter().copied().collect();
        let mut packets = Vec::with_capacity(50);

        for key in keys {
            if let Some(player) = store.players.get_mut(&key) {
                let task = player.lock().await.switch_tasks.take();
                if let Some(mut task) = task {
                    let mut count = task.npcs.len() + task.players.len() + task.items.len();

                    for npc in task.npcs.drain(..task.npcs.len().min(50)) {
                        count -= 1;
                        packets.push(npc_spawn_packet(&npc, false)?);
                    }

                    DataTaskToken::NpcSpawnToEntity(key).add_tasks(self, &mut packets)?;
                    packets.clear();

                    for player in task.players.drain(..task.players.len().min(50)) {
                        count -= 1;
                        packets.push(player_spawn_packet(&player, false)?);
                    }

                    DataTaskToken::PlayerSpawnToEntity(key).add_tasks(self, &mut packets)?;
                    packets.clear();

                    for item in task.items.drain(..task.items.len().min(50)) {
                        count -= 1;

                        packets.push(map_item_packet(&item, false)?);
                    }

                    DataTaskToken::ItemLoadToEntity(key).add_tasks(self, &mut packets)?;

                    if count == 0 {
                        self.player_switch_processing.swap_remove(&key);
                    }

                    let _ = player.lock().await.switch_tasks.insert(task);
                }
            }
        }

        Ok(())
    }
}
