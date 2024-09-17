use std::cmp::max;

use crate::{
    containers::*,
    gametypes::*,
    maps::*,
    players::*,
    tasks::{map_item_packet, npc_spawn_packet, player_spawn_packet, DataTaskToken},
};

//types to buffer load when loading a map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapSwitchTasks {
    Npc(Vec<GlobalKey>),    //0
    Player(Vec<GlobalKey>), //1
    Items(Vec<GlobalKey>),  //2
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlayerSwitchTasks {
    //So we know we got events back from the maps we sent them before disposing of this data.
    requests: u64,
    npcs: Vec<Npc>,
    players: Vec<Player>,
    items: Vec<MapItem>,
}

const PROCESS_LIMIT: usize = 1000;

pub async fn process_data_lists(player: &mut Player) -> Result<()> {
    if let Some(task) = player.switch_tasks {
        let mut count = task.npcs.len() + task.players.len() + task.items.len();


        for npc in task.npcs.drain(..task.npcs.len().min(50)) {
                DataTaskToken::NpcSpawnToEntity(socket_id)
                    .add_task(
                        storage,
                        npc_spawn_packet(world, &entity, false).await?,
                    )
                    .await?;
        }


        if let Ok(socket_id) = socket_id {
            for task in tasks {
                let amount_left = match task {
                    MapSwitchTasks::Npc(entities) => {
                        let cursor = entities.len().saturating_sub(process_limit);

                        for entity in entities.drain(cursor..) {
                            if world.contains(&entity).await {
                                DataTaskToken::NpcSpawnToEntity(socket_id)
                                    .add_task(
                                        storage,
                                        npc_spawn_packet(world, &entity, false).await?,
                                    )
                                    .await?;
                            }
                        }

                        entities.len()
                    }
                    MapSwitchTasks::Player(entities) => {
                        let cursor = entities.len().saturating_sub(process_limit);

                        for entity in entities.drain(cursor..) {
                            if world.contains(&entity).await {
                                DataTaskToken::PlayerSpawnToEntity(socket_id)
                                    .add_task(
                                        storage,
                                        player_spawn_packet(world, &entity, false).await?,
                                    )
                                    .await?;
                            }
                        }

                        entities.len()
                    }
                    MapSwitchTasks::Items(entities) => {
                        let cursor = entities.len().saturating_sub(process_limit);

                        for entity in entities.drain(cursor..) {
                            let lock = world.read().await;
                            let map_item = lock.get::<&MapItem>(entity.0);
                            if let Ok(map_item) = map_item {
                                DataTaskToken::ItemLoadToEntity(socket_id)
                                    .add_task(
                                        storage,
                                        map_item_packet(
                                            entity,
                                            map_item.pos,
                                            map_item.item,
                                            map_item.ownerid,
                                            false,
                                        )?,
                                    )
                                    .await?;
                            }
                        }

                        entities.len()
                    }
                };

                if amount_left > 0 {
                    contains_data = true;
                }
            }
        }
    }

    Ok(())
}
