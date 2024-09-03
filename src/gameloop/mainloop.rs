use crate::{
    containers::{GameStore, GameWorld},
    maps::{update_map_items, update_maps},
    npcs::*,
    players::*,
    socket::*,
    tasks::{process_data_lists, process_tasks},
    time_ext::MyInstant,
};
use chrono::Duration;
use log::info;

pub async fn game_loop(world: &GameWorld, storage: &GameStore) {
    let mut tick: MyInstant;
    let mut tmr100: MyInstant = MyInstant::now();
    let mut tmr500: MyInstant = MyInstant::now();
    let mut tmr1000: MyInstant = MyInstant::now();
    let mut tmr60000: MyInstant = MyInstant::now();
    let mut ping_timer: MyInstant = MyInstant::now();

    loop {
        {
            *storage.gettick.write().await = MyInstant::now();
        };
        tick = *storage.gettick.read().await;

        if tick > tmr100 {
            update_npcs(world, storage).await.unwrap();
            update_players(world, storage).await.unwrap();
            update_map_items(world, storage).await.unwrap();
            check_player_connection(world, storage).await.unwrap();
            tmr100 = tick + Duration::try_milliseconds(100).unwrap_or_default();
        }

        if tick > tmr500 {
            update_maps(world, storage).await.unwrap();
            tmr500 = tick + Duration::try_milliseconds(500).unwrap_or_default();
        }

        if tick > tmr1000 {
            tmr1000 = tick + Duration::try_milliseconds(1000).unwrap_or_default();
        }

        if tick > tmr60000 {
            let mut time = storage.time.write().await;
            time.min += 1;
            if time.min >= 60 {
                time.min = 0;
                time.hour += 1;
                if time.hour >= 24 {
                    time.hour = 0;
                }
            }
            tmr60000 = tick + Duration::try_milliseconds(60000).unwrap_or_default();
        }

        //to ping sockets to ensure they are still connected. we will then unload those who are not.
        if tick > ping_timer {
            send_connection_pings(world, storage).await.unwrap();
            ping_timer = tick + Duration::try_hours(2).unwrap_or_default();
        }

        //poll_events(world, storage).await.unwrap();
        //process_packets(world, storage).await.unwrap();
        //process_data_lists(world, storage).await.unwrap();
        //process_tasks(world, storage).await.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
