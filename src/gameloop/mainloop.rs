use crate::{
    containers::{Storage, World},
    maps::{update_map_items, update_maps},
    npcs::*,
    players::*,
    socket::*,
    tasks::{process_data_lists, process_tasks},
};
use chrono::Duration;
use time::Instant;

pub fn game_loop(world: &mut World, storage: &Storage) {
    let mut tick: Instant;
    let mut tmr100: Instant = Instant::recent();
    let mut tmr150: Instant = Instant::recent();
    let mut tmr500: Instant = Instant::recent();
    let mut tmr1000: Instant = Instant::recent();
    let mut tmr60000: Instant = Instant::recent();
    let mut ping_timer: Instant = Instant::recent();

    let mut entity_progress = 0u64;
    let mut npc_progress = 0u64;
    let mut npc_batch = 0usize;
    let mut max_batch = (storage.npc_ids.borrow().len() as f32 / 5.0).ceil() as usize;

    loop {
        let _ = storage.gettick.replace(Instant::recent());
        tick = *storage.gettick.borrow();

        if tick > tmr100 {
            match entity_progress {
                1 => {
                    check_player_connection(world, storage).unwrap();
                }
                2 => {
                    update_map_items(world, storage).unwrap();
                }
                _ => {
                    update_players(world, storage).unwrap();
                    entity_progress = 0;
                    tmr100 = tick + Duration::try_milliseconds(100).unwrap_or_default();
                }
            }

            entity_progress += 1;
        }

        if tick > tmr150 {
            if npc_batch >= max_batch {
                unload_npcs(world, storage).unwrap();

                max_batch = (storage.npc_ids.borrow().len() as f32 / 5.0).ceil() as usize;
                npc_batch = 0;
                tmr150 = tick + Duration::try_milliseconds(150).unwrap_or_default();
            } else {
                match npc_progress {
                    1 => {
                        update_npcs_targetting(world, storage, npc_batch).unwrap();
                    }
                    2 => {
                        update_npcs_movement(world, storage, npc_batch).unwrap();
                    }
                    3 => {
                        update_npcs_combat(world, storage, npc_batch).unwrap();
                    }
                    _ => {
                        update_npcs_spawn(world, storage, npc_batch).unwrap();
                        npc_progress = 0;
                        npc_batch += 1;
                    }
                }

                npc_progress += 1;
            }
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
        process_packets(world, storage).unwrap();
        process_data_lists(world, storage).unwrap();
        process_tasks(world, storage).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
