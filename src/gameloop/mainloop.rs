use crate::{
    containers::Storage, gameloop::handle_data, maps::update_maps, npcs::*, players::*, socket::*,
    time_ext::MyInstant,
};
use chrono::Duration;
use hecs::{Entity, World};

pub fn game_loop(world: &mut World, storage: &Storage) {
    let mut tick: MyInstant;
    let mut tmr100: MyInstant = MyInstant::now();
    let mut tmr500: MyInstant = MyInstant::now();
    let mut tmr1000: MyInstant = MyInstant::now();
    let mut tmr60000: MyInstant = MyInstant::now();

    loop {
        let _ = storage.gettick.replace(MyInstant::now());
        tick = *storage.gettick.borrow();

        if tick > tmr100 {
            update_npcs(world);
            update_players(world, storage);
            tmr100 = tick + Duration::milliseconds(100);
        }

        if tick > tmr500 {
            if let Err(e) = update_maps(storage) {
                println!("Error: {:?}", e);
            }
            tmr500 = tick + Duration::milliseconds(500);
        }

        if tick > tmr1000 {
            tmr1000 = tick + Duration::milliseconds(1000);
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
            tmr60000 = tick + Duration::milliseconds(60000);
        }

        if let Err(e) = poll_events(world, storage) {
            println!("Poll event error: {:?}", e);
        }

        process_packets(world, storage);
    }
}

pub fn get_length(world: &Storage, buffer: &mut ByteBuffer, id: usize) -> Option<u64> {
    if buffer.length() - buffer.cursor() >= 8 {
        let length = buffer.read::<u64>().ok()?;

        if !(4..=8192).contains(&length) {
            if let Some(mut client) = world.server.borrow().get_mut(mio::Token(id)) {
                client.set_to_closing(world);
            }
        }

        Some(length)
    } else {
        if let Some(mut client) = world.server.borrow().get_mut(mio::Token(id)) {
            client.poll_state.add(SocketPollState::Read);
            client.reregister(&world.poll.borrow_mut()).unwrap();
        }

        None
    }
}

pub fn process_packets(world: &hecs::World, storage: &Storage) {
    let mut count: usize;
    let mut rem_arr: Vec<usize> = Vec::with_capacity(32);
    let mut length: u64;

    'user_loop: for i in &*storage.recv_ids.borrow() {
        count = 0;

        if let Some(player) = storage.players.borrow().get(*i) {
            let socket_id = player.borrow().socket_id;

            loop {
                length = match get_length(world, &mut player.borrow_mut().buffer, socket_id) {
                    Some(n) => n,
                    None => {
                        rem_arr.push(*i);
                        continue 'user_loop;
                    }
                };

                if length > 0
                    && length
                        <= (player.borrow().buffer.length() - player.borrow().buffer.cursor())
                            as u64
                {
                    let mut buffer =
                        match player.borrow_mut().buffer.read_to_buffer(length as usize) {
                            Some(n) => n,
                            None => {
                                if let Some(mut client) =
                                    storage.server.borrow().get_mut(mio::Token(socket_id))
                                {
                                    client.set_to_closing(world, storage);
                                }

                                rem_arr.push(*i);
                                continue 'user_loop;
                            }
                        };

                    if handle_data(world, storage, &mut buffer, *i).is_err() {
                        if let Some(mut client) =
                            storage.server.borrow().get_mut(mio::Token(socket_id))
                        {
                            client.set_to_closing(world, storage);
                        }

                        rem_arr.push(*i);
                        continue 'user_loop;
                    }

                    count += 1
                } else {
                    let _ = player
                        .borrow_mut()
                        .buffer
                        .move_cursor(player.borrow().buffer.cursor() - 8);

                    if let Some(mut client) =
                        storage.server.borrow_mut().get_mut(mio::Token(socket_id))
                    {
                        client.poll_state.add(SocketPollState::Read);
                        client.reregister(&storage.poll.borrow_mut()).unwrap();
                    }

                    rem_arr.push(*i);
                    break;
                }

                if count == 25 {
                    break;
                }
            }

            if player.borrow().buffer.cursor() == player.borrow().buffer.length() {
                let _ = player.borrow_mut().buffer.truncate(0);
            }

            if player.borrow().buffer.capacity() > 25000 {
                let _ = player.borrow_mut().buffer.resize(4096);
            }
        }
    }

    for i in rem_arr {
        storage.recv_ids.borrow_mut().remove(&i);
    }
}
