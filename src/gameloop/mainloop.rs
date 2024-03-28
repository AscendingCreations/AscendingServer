use crate::{
    containers::Storage,
    gameloop::handle_data,
    gametypes::*,
    maps::update_maps,
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

    loop {
        let _ = storage.gettick.replace(MyInstant::now());
        tick = *storage.gettick.borrow();

        if tick > tmr100 {
            update_npcs(world, storage).unwrap();
            update_players(world, storage).unwrap();
            tmr100 = tick + Duration::try_milliseconds(100).unwrap_or_default();
        }

        if tick > tmr500 {
            if let Err(e) = update_maps(world, storage) {
                println!("Error: {:?}", e);
            }
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

        if let Err(e) = poll_events(world, storage) {
            println!("Poll event error: {:?}", e);
        }

        process_packets(world, storage, router).unwrap();
        process_data_lists(world, storage).unwrap();
        process_tasks(world, storage).unwrap();
    }
}

pub fn get_length(storage: &Storage, buffer: &mut ByteBuffer, id: usize) -> Result<Option<u64>> {
    if buffer.length() - buffer.cursor() >= 8 {
        let length = buffer.read::<u64>()?;

        if !(1..=8192).contains(&length) {
            if let Some(client) = storage.server.borrow().clients.get(&mio::Token(id)) {
                client.borrow_mut().set_to_closing(storage)?;
                return Ok(None);
            }
        }

        Ok(Some(length))
    } else {
        if let Some(client) = storage.server.borrow().clients.get(&mio::Token(id)) {
            client.borrow_mut().poll_state.add(SocketPollState::Read);
            client.borrow_mut().reregister(&storage.poll.borrow_mut())?;
        }

        Ok(None)
    }
}

pub fn process_packets(world: &mut World, storage: &Storage, router: &PacketRouter) -> Result<()> {
    let mut count: usize;
    let mut rem_arr: Vec<Entity> = Vec::with_capacity(32);
    let mut length: u64;

    'user_loop: for entity in &*storage.recv_ids.borrow() {
        count = 0;

        let (lock, socket_id) = {
            let socket = world.get::<&Socket>(entity.0)?;

            (socket.buffer.clone(), socket.id)
        };

        if let Ok(mut buffer) = lock.lock() {
            loop {
                length = match get_length(storage, &mut buffer, socket_id)? {
                    Some(n) => n,
                    None => {
                        rem_arr.push(*entity);
                        continue 'user_loop;
                    }
                };

                if length > 0 && length <= (buffer.length() - buffer.cursor()) as u64 {
                    let mut buffer = match buffer.read_to_buffer(length as usize) {
                        Ok(n) => n,
                        Err(_) => {
                            if let Some(client) =
                                storage.server.borrow().clients.get(&mio::Token(socket_id))
                            {
                                client.borrow_mut().set_to_closing(storage)?;
                            }

                            rem_arr.push(*entity);
                            continue 'user_loop;
                        }
                    };

                    if handle_data(router, world, storage, &mut buffer, entity).is_err() {
                        if let Some(client) =
                            storage.server.borrow().clients.get(&mio::Token(socket_id))
                        {
                            client.borrow_mut().set_to_closing(storage)?;
                        }

                        rem_arr.push(*entity);
                        continue 'user_loop;
                    }

                    count += 1
                } else {
                    let cursor = buffer.cursor() - 8;
                    buffer.move_cursor(cursor)?;

                    rem_arr.push(*entity);
                    break;
                }

                if count == 25 {
                    break;
                }
            }

            if buffer.cursor() == buffer.length() {
                buffer.truncate(0)?;
            }

            if buffer.capacity() > 25000 {
                buffer.resize(4096)?;
            }
        };
    }

    for i in rem_arr {
        storage.recv_ids.borrow_mut().swap_remove(&i);
    }

    Ok(())
}
