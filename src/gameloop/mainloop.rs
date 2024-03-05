use crate::{
    containers::Storage, gameloop::handle_data, gametypes::*, maps::update_maps, npcs::*,
    players::*, socket::*, time_ext::MyInstant,
};
use chrono::Duration;
use hecs::World;

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
            update_npcs(world, storage);
            update_players(world, storage);
            tmr100 = tick + Duration::milliseconds(100);
        }

        if tick > tmr500 {
            if let Err(e) = update_maps(world, storage) {
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

pub fn get_length(storage: &Storage, buffer: &mut ByteBuffer, id: usize) -> Option<u64> {
    if buffer.length() - buffer.cursor() >= 8 {
        let length = buffer.read::<u64>().ok()?;

        if !(4..=8192).contains(&length) {
            if let Some(client) = storage.server.borrow().clients.get(&mio::Token(id)) {
                client.borrow_mut().set_to_closing(storage);
            }
        }

        Some(length)
    } else {
        if let Some(client) = storage.server.borrow().clients.get(&mio::Token(id)) {
            client.borrow_mut().poll_state.add(SocketPollState::Read);
            client
                .borrow_mut()
                .reregister(&storage.poll.borrow_mut())
                .unwrap();
        }

        None
    }
}

pub fn process_packets(world: &mut World, storage: &Storage) {
    let mut count: usize;
    let mut rem_arr: Vec<Entity> = Vec::with_capacity(32);
    let mut length: u64;

    'user_loop: for entity in &*storage.recv_ids.borrow() {
        count = 0;

        //if let Some(player) = storage.players.borrow().get(*i) {
        /*for (entity, (_, socket)) in world
            .query::<((&WorldEntityType, &OnlineType), &Socket)>()
            .iter()
            .filter(|(_entity,
                ((worldentitytype, onlinetype), _))| {
                **worldentitytype == WorldEntityType::Player && **onlinetype == OnlineType::Online
            })
        {*/
        if !is_player_online(world, entity) {
            continue;
        }

        let (lock, socket_id) = {
            let socket = world.get_or_panic::<&Socket>(entity);

            (socket.buffer.clone(), socket.id)
        };

        let mut buffer = lock.lock().unwrap();

        loop {
            length = match get_length(storage, &mut buffer, socket_id) {
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
                            client.borrow_mut().set_to_closing(storage);
                        }

                        rem_arr.push(*entity);
                        continue 'user_loop;
                    }
                };

                if handle_data(world, storage, &mut buffer, entity).is_err() {
                    if let Some(client) =
                        storage.server.borrow().clients.get(&mio::Token(socket_id))
                    {
                        client.borrow_mut().set_to_closing(storage);
                    }

                    rem_arr.push(*entity);
                    continue 'user_loop;
                }

                count += 1
            } else {
                let cursor = buffer.cursor() - 8;
                let _ = buffer.move_cursor(cursor);

                if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket_id)) {
                    client.borrow_mut().poll_state.add(SocketPollState::Read);
                    client
                        .borrow_mut()
                        .reregister(&storage.poll.borrow_mut())
                        .unwrap();
                }

                rem_arr.push(*entity);
                break;
            }

            if count == 25 {
                break;
            }
        }

        if buffer.cursor() == buffer.length() {
            let _ = buffer.truncate(0);
        }

        if buffer.capacity() > 25000 {
            let _ = buffer.resize(4096);
        }
    }

    for i in rem_arr {
        storage.recv_ids.borrow_mut().swap_remove(&i);
    }
}
