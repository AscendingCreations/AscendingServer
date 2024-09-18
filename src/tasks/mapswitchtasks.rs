use std::cmp::max;

use crate::{
    containers::*,
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
        let keys: Vec<GlobalKey> = store.player_switch_processing.iter().copied().collect();

        for key in keys {
            let rc_player = store.players.get(&key).cloned();
            if let Some(player) = rc_player {
                let task = player.borrow_mut().switch_tasks.take();
                if let Some(mut task) = task {
                    let mut count = task.npcs.len() + task.players.len() + task.items.len();

                    for npc in task.npcs.drain(..task.npcs.len().min(50)) {
                        count -= 1;
                        DataTaskToken::NpcSpawnToEntity(key)
                            .add_task(store, npc_spawn_packet(&npc, false)?)
                            .await?;
                    }

                    for player in task.players.drain(..task.players.len().min(50)) {
                        count -= 1;
                        DataTaskToken::PlayerSpawnToEntity(key)
                            .add_task(store, player_spawn_packet(&player, false)?)
                            .await?;
                    }

                    for item in task.items.drain(..task.items.len().min(50)) {
                        count -= 1;
                        DataTaskToken::ItemLoadToEntity(key)
                            .add_task(store, map_item_packet(&item, false)?)
                            .await?;
                    }

                    if count == 0 {
                        store.player_switch_processing.swap_remove(&key);
                    }

                    let _ = player.borrow_mut().switch_tasks.insert(task);
                }
            }
        }

        Ok(())
    }
}
