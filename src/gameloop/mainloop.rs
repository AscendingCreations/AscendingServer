use crate::{
    containers::Storage,
    maps::{update_map_items, update_maps},
    npcs::*,
    players::*,
    socket::*,
    tasks::{process_data_lists, process_tasks},
    time_ext::MyInstant,
    PacketRouter,
};
use chrono::Duration;
use hecs::World;

pub fn game_loop(world: &mut World, storage: &Storage, router: &PacketRouter) {
    let mut tick: MyInstant;
    let mut tmr100: MyInstant = MyInstant::now();
    let mut tmr500: MyInstant = MyInstant::now();
    let mut tmr1000: MyInstant = MyInstant::now();
    let mut tmr60000: MyInstant = MyInstant::now();
    let mut ping_timer: MyInstant = MyInstant::now();

    loop {
        let _ = storage.gettick.replace(MyInstant::now());
        tick = *storage.gettick.borrow();

        if tick > tmr100 {
            update_npcs(world, storage).unwrap();
            update_players(world, storage).unwrap();
            update_map_items(world, storage).unwrap();
            check_player_connection(world, storage).unwrap();
            tmr100 = tick + Duration::try_milliseconds(100).unwrap_or_default();
        }

        if tick > tmr500 {
            update_maps(world, storage).unwrap();
            tmr500 = tick + Duration::try_milliseconds(500).unwrap_or_default();
        }

        if tick > tmr1000 {
            tmr1000 = tick + Duration::try_milliseconds(1000).unwrap_or_default();
        }

        if tick > tmr60000 {
            let mut time = storage.time.borrow_mut();
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
            send_connection_pings(world, storage).unwrap();
            ping_timer = tick + Duration::try_hours(2).unwrap_or_default();
        }

        poll_events(world, storage).unwrap();
        process_packets(world, storage, router).unwrap();
        process_data_lists(world, storage).unwrap();
        process_tasks(world, storage).unwrap();
    }
}
