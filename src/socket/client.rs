use crate::{
    containers::{Entity, GlobalKey, Storage, World},
    gameloop::SocketID,
    gametypes::*,
    handle_data,
    maps::*,
    players::*,
    socket::*,
    sql::save_player,
    tasks::{DataTaskToken, unload_entity_packet},
};
use chrono::Duration;
use log::{error, info, trace, warn};
use mio::{Token, net::TcpStream};
use mmap_bytey::BUFFER_SIZE;
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct Client {
    pub stream: TcpStream,
    pub token: mio::Token,
    pub entity: Option<GlobalKey>,
    pub state: ClientState,
    pub poll_state: PollState,
    pub sends: VecDeque<MByteBuffer>,
    // used for sending encrypted Data.
    pub tls: Option<rustls::ServerConnection>,
    pub buffer: Arc<Mutex<ByteBuffer>>,
    pub addr: Arc<String>,
}

impl Client {
    #[inline]
    pub fn new(
        stream: TcpStream,
        token: mio::Token,
        tls: Option<rustls::ServerConnection>,
        addr: String,
    ) -> Result<Client> {
        Ok(Client {
            stream,
            token,
            entity: None,
            state: ClientState::Open,
            poll_state: PollState::ReadWrite,
            sends: VecDeque::with_capacity(32),
            tls,
            buffer: Arc::new(Mutex::new(ByteBuffer::with_capacity(8192)?)),
            addr: Arc::new(addr),
        })
    }

    pub fn process(
        &mut self,
        event: &mio::event::Event,
        world: &mut World,
        storage: &Storage,
    ) -> Result<()> {
        // Check if the Event has some readable Data from the Poll State.
        if event.is_readable() {
            if self.tls.is_some() {
                self.tls_read(storage)?;
            } else {
                self.read(storage)?;
            }
        }

        // Check if the Event has some writable Data from the Poll State.
        if event.is_writable() {
            if self.tls.is_some() {
                self.tls_write();
            } else {
                self.write();
            }
        }

        // Check if the Socket is closing if not lets reregister the poll event for it.
        // if `SocketPollState::None` is registers as the poll event we will not get data.
        match self.state {
            ClientState::Closing => self.close_socket(world, storage)?,
            _ => self.reregister(&storage.poll.borrow_mut())?,
        }

        Ok(())
    }

    #[inline]
    pub fn deregister(&mut self, poll: &mio::Poll) -> Result<()> {
        Ok(poll.registry().deregister(&mut self.stream)?)
    }

    /// Use this to disconnect the player from socket without actually disconnecting the socket.
    pub fn set_player(&mut self, entity: Option<GlobalKey>) {
        self.entity = entity;
    }

    #[inline]
    pub fn set_to_closing(&mut self) {
        self.state = ClientState::Closing;
    }

    #[inline]
    pub fn close_socket(&mut self, world: &mut World, storage: &Storage) -> Result<()> {
        match self.state {
            ClientState::Closed => Ok(()),
            _ => {
                //We dont care about errors here as they only occur when a socket is already disconnected by the client.
                self.deregister(&storage.poll.borrow_mut())?;
                self.state = ClientState::Closed;
                let _ = self.stream.shutdown(std::net::Shutdown::Both);

                let mut remove_entity = false;

                if let Some(entity) = self.entity
                    && let Some(Entity::Player(data)) = world.get_opt_entity(entity)
                {
                    let mut data = data.try_lock()?;

                    if self.tls.is_some() {
                        data.socket.tls_id = usize::MAX;
                        println!("TLS Socket unloaded");
                    } else {
                        data.socket.id = usize::MAX;
                        println!("Socket unloaded");
                    }
                    remove_entity = true;

                    let tls_connected = data.socket.tls_id != usize::MAX;
                    let non_tls_connected = data.socket.id != usize::MAX;

                    if !non_tls_connected
                        && (data.online_type == OnlineType::Online || !tls_connected)
                    {
                        let _ = storage.disconnected_player.borrow_mut().insert(entity);

                        info!(
                            "Added player on disconnected list : {}",
                            &data.account.username
                        );

                        data.connection.disconnect_timer = *storage.gettick.borrow()
                            + Duration::try_milliseconds(60000).unwrap_or_default();
                    }
                }

                if remove_entity {
                    self.entity = None;
                }

                Ok(())
            }
        }
    }

    pub fn tls_read(&mut self, storage: &Storage) -> Result<()> {
        let tls = match &mut self.tls {
            Some(v) => v,
            None => {
                //this should never get called...
                self.state = ClientState::Closing;
                return Ok(());
            }
        };

        let arc_buffer = self.buffer.clone();
        // get the current pos so we can reset it back for reading.
        let mut buffer = arc_buffer.lock().unwrap();
        let pos = buffer.cursor();
        buffer.move_cursor_to_end();

        loop {
            match tls.read_tls(&mut self.stream) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(error) => {
                    error!("TLS read error: {error:?}");
                    self.state = ClientState::Closing;
                    buffer.move_cursor(pos)?;
                    return Ok(());
                }
                Ok(0) => {
                    trace!("Client side socket closed");
                    self.state = ClientState::Closing;
                    buffer.move_cursor(pos)?;
                    return Ok(());
                }
                Ok(_) => {}
            }

            let io_state = match tls.process_new_packets() {
                Ok(io_state) => io_state,
                Err(err) => {
                    error!("TLS error: {err:?}");
                    self.state = ClientState::Closing;
                    buffer.move_cursor(pos)?;
                    return Ok(());
                }
            };

            if io_state.plaintext_bytes_to_read() > 0 {
                let mut buf = vec![0u8; io_state.plaintext_bytes_to_read()];
                if let Err(e) = tls.reader().read_exact(&mut buf) {
                    trace!("TLS read error: {e}");
                    self.state = ClientState::Closing;
                    buffer.move_cursor(pos)?;
                    return Ok(());
                }

                if let Err(e) = buffer.write_slice(&buf) {
                    trace!("TLS read buffer write error: {e}");
                    self.state = ClientState::Closing;
                    buffer.move_cursor(pos)?;
                    return Ok(());
                }
            }

            if io_state.peer_has_closed() {
                trace!("TLS peer has closed");
                self.state = ClientState::Closing;
            }

            break;
        }

        // reset it back to the original pos so we can Read from it again.
        buffer.move_cursor(pos)?;

        if !buffer.is_empty() {
            storage
                .recv_ids
                .borrow_mut()
                .insert(self.token.0 - CLIENT_OFFSET);
        }

        Ok(())
    }

    pub fn read(&mut self, storage: &Storage) -> Result<()> {
        let arc_buffer = self.buffer.clone();
        // get the current pos so we can reset it back for reading.
        let mut buffer = arc_buffer.lock().unwrap();
        let pos = buffer.cursor();
        buffer.move_cursor_to_end();

        let mut buf: [u8; 4096] = [0; 4096];
        let mut closing = false;

        loop {
            match self.stream.read(&mut buf) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Ok(0) => closing = true,
                Err(e) => {
                    trace!("stream.read, error in socket read: {e}");
                    closing = true;
                }
                Ok(n) => {
                    if let Err(e) = buffer.write_slice(&buf[0..n]) {
                        trace!("buffer.write_slice, error in socket read: {e}");
                        closing = true;
                    }
                }
            }

            if closing {
                // We are closing the socket so we dont need to handle it again.
                self.state = ClientState::Closing;
                return Ok(());
            }
        }

        // reset it back to the original pos so we can Read from it again.
        buffer.move_cursor(pos)?;

        if !buffer.is_empty() {
            storage
                .recv_ids
                .borrow_mut()
                .insert(self.token.0 - CLIENT_OFFSET);
        }

        Ok(())
    }

    pub fn write(&mut self) {
        let mut count: usize = 0;

        //info!("Player sends count: {}", self.sends.len());
        // lets only send 25 packets per socket each loop.
        while count < 25 {
            let mut packet = match self.sends.pop_front() {
                Some(packet) => packet,
                None => {
                    if self.sends.capacity() > 100 && self.sends.len() < 50 {
                        warn!(
                            "Socket write: sends Buffer Strink to 100, Current Capacity {}, Current len {}.",
                            self.sends.capacity(),
                            self.sends.len()
                        );
                        self.sends.shrink_to(100);
                    }
                    return;
                }
            };

            match self.stream.write_all(packet.as_slice()) {
                Ok(()) => count += 1,
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    //Operation would block so we insert it back in to try again later.
                    self.sends.push_front(packet);
                    break;
                }
                Err(e) => {
                    trace!("stream.write_all error in socket write: {e}");
                    self.state = ClientState::Closing;
                    return;
                }
            }
        }

        if !self.sends.is_empty() {
            self.poll_state.add(PollState::Write);
        } else {
            self.poll_state.remove(PollState::Write);
        }
    }

    pub fn tls_write(&mut self) {
        let tls = match &mut self.tls {
            Some(v) => v,
            None => {
                //this should never get called...
                self.state = ClientState::Closing;
                return;
            }
        };

        // lets only send 25 packets per socket each loop.
        loop {
            let mut packet = match self.sends.pop_front() {
                Some(packet) => packet,
                None => {
                    if self.sends.capacity() > 100 {
                        warn!(
                            "Socket TLSwrite: tls_sends Buffer Strink to 100, Current Capacity {}, Current len {}.",
                            self.sends.capacity(),
                            self.sends.len()
                        );
                        self.sends.shrink_to(100);
                    }
                    break;
                }
            };

            match tls.writer().write_all(packet.as_slice()) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.sends.push_front(packet);
                    break;
                }
                Err(e) => {
                    trace!("tls write, error in write_all: {e}");
                    self.state = ClientState::Closing;
                    return;
                }
                Ok(_) => {}
            }
        }

        loop {
            if tls.wants_write() {
                match tls.write_tls(&mut self.stream) {
                    Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                        continue;
                    }
                    Err(e) => {
                        trace!("tls write, error in write_tls: {e}");
                        self.state = ClientState::Closing;
                        return;
                    }
                    Ok(_) => {}
                };
            } else {
                break;
            }
        }
    }

    #[inline]
    pub fn register(&mut self, poll: &mio::Poll) -> Result<()> {
        poll.registry()
            .register(&mut self.stream, self.token, self.poll_state.to_interest())?;
        Ok(())
    }

    #[inline]
    pub fn reregister(&mut self, poll: &mio::Poll) -> Result<()> {
        if self.state == ClientState::Open {
            poll.registry().reregister(
                &mut self.stream,
                self.token,
                self.poll_state.to_interest(),
            )?;
        }

        Ok(())
    }

    #[inline]
    pub fn send(&mut self, poll: &mio::Poll, buf: MByteBuffer) -> Result<()> {
        self.sends.push_back(buf);
        self.poll_state.add(PollState::Write);
        self.reregister(poll)
    }

    #[inline]
    pub fn send_first(&mut self, poll: &mio::Poll, buf: MByteBuffer) -> Result<()> {
        self.sends.push_front(buf);
        self.poll_state.add(PollState::Write);
        self.reregister(poll)
    }
}

#[inline]
pub fn disconnect(playerid: GlobalKey, world: &mut World, storage: &Storage) -> Result<()> {
    left_game(world, storage, playerid)?;

    let _ = storage
        .disconnected_player
        .borrow_mut()
        .swap_remove(&playerid);
    let _ = storage.player_timeout.borrow_mut().remove(playerid);

    let position = if let Some(player) = storage.remove_player(world, playerid)? {
        let pos = {
            let player = player.try_lock()?;

            trace!("Players Disconnected IP: {} ", &player.socket.addr);

            player.movement.pos
        };

        save_player(storage, player)?;

        Some(pos)
    } else {
        None
    };

    if let Some(pos) = position
        && let Some(map) = storage.maps.get(&pos.map)
    {
        map.borrow_mut().remove_player(storage, playerid);
        map.borrow_mut().remove_entity_from_grid(pos);
        DataTaskToken::EntityUnload(pos.map).add_task(storage, unload_entity_packet(playerid)?)?;
    }

    Ok(())
}

#[inline]
pub fn send_to(storage: &Storage, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage.server.borrow().clients.get(socket_id) {
        client.borrow_mut().send(&storage.poll.borrow(), buf)
    } else {
        Ok(())
    }
}

#[inline]
pub fn send_to_front(storage: &Storage, socket_id: Token, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage
        .server
        .borrow()
        .clients
        .get(socket_id.0 - CLIENT_OFFSET)
    {
        client.borrow_mut().send_first(&storage.poll.borrow(), buf)
    } else {
        Ok(())
    }
}

#[inline]
pub fn send_to_all(world: &mut World, storage: &Storage, buf: MByteBuffer) -> Result<()> {
    for (_, entity) in world.entities.iter() {
        if let Entity::Player(data) = entity {
            let data = data.try_lock()?;

            if data.online_type == OnlineType::Online
                && let Some(client) = storage.server.borrow().clients.get(data.socket.id)
            {
                client
                    .borrow_mut()
                    .send(&storage.poll.borrow(), buf.try_clone()?)?;
            }
        }
    }

    Ok(())
}

#[inline]
pub fn send_to_maps(
    world: &mut World,
    storage: &Storage,
    position: MapPosition,
    buf: MByteBuffer,
    avoidindex: Option<GlobalKey>,
) -> Result<()> {
    for m in get_surrounding(position, true) {
        let map = match storage.maps.get(&m) {
            Some(map) => map,
            None => continue,
        }
        .borrow();

        for entity in &map.players {
            if avoidindex.map(|value| value == *entity).unwrap_or(false) {
                continue;
            }

            if let Some(Entity::Player(data)) = world.get_opt_entity(*entity) {
                let data = data.try_lock()?;

                if data.online_type == OnlineType::Online
                    && let Some(client) = storage.server.borrow().clients.get(data.socket.id)
                {
                    client
                        .borrow_mut()
                        .send(&storage.poll.borrow(), buf.try_clone()?)?;
                }
            }
        }
    }

    Ok(())
}

#[inline]
pub fn send_to_entities(
    world: &mut World,
    storage: &Storage,
    entities: &[GlobalKey],
    buf: MByteBuffer,
) -> Result<()> {
    for entity in entities {
        if let Some(Entity::Player(data)) = world.get_opt_entity(*entity) {
            let data = data.try_lock()?;

            if data.online_type == OnlineType::Online
                && let Some(client) = storage.server.borrow().clients.get(data.socket.id)
            {
                client
                    .borrow_mut()
                    .send(&storage.poll.borrow(), buf.try_clone()?)?;
            }
        }
    }

    Ok(())
}

pub fn get_length(storage: &Storage, buffer: &mut ByteBuffer, token: usize) -> Result<Option<u64>> {
    if buffer.length() - buffer.cursor() >= 8 {
        let length = buffer.read::<u64>()?;

        if !(1..=8192).contains(&length)
            && let Some(client) = storage.server.borrow().clients.get(token)
        {
            trace!("Player was disconnected on get_length LENGTH: {length:?}");
            client.borrow_mut().set_to_closing();
            return Ok(None);
        }

        Ok(Some(length))
    } else {
        Ok(None)
    }
}

pub fn set_client_as_closed(storage: &Storage, token: usize) {
    if let Some(client) = storage.server.borrow().clients.get(token) {
        client.borrow_mut().set_to_closing();
    }
}

pub const MAX_PROCESSED_PACKETS: i32 = 25;

pub fn process_packets(world: &mut World, storage: &Storage) -> Result<()> {
    let mut packet = MByteBuffer::new()?;
    let mut rerun: Vec<usize> = Vec::with_capacity(64);

    'user_loop: for token in storage.recv_ids.borrow_mut().drain(..) {
        let mut count = 0;

        let (lock, entity, address, is_tls) = {
            match storage.server.borrow().clients.get(token) {
                Some(v) => {
                    let brw_client = v.borrow();
                    (
                        brw_client.buffer.clone(),
                        brw_client.entity,
                        brw_client.addr.clone(),
                        brw_client.tls.is_some(),
                    )
                }
                None => {
                    error!("Socket was missing in server clients.");
                    continue 'user_loop;
                }
            }
        };

        if let Ok(mut buffer) = lock.lock() {
            loop {
                packet.move_cursor_to_start();
                let length = match get_length(storage, &mut buffer, token)? {
                    Some(n) => n,
                    None => {
                        break;
                    }
                };

                if length == 0 {
                    trace!("Length was Zero. Bad or malformed packet from IP: {address}");
                    set_client_as_closed(storage, token);
                    continue 'user_loop;
                }

                if length > BUFFER_SIZE as u64 {
                    trace!(
                        "Length was {length} greater than the max packet size of {address}. Bad or malformed packet from IP: {BUFFER_SIZE}"
                    );
                    set_client_as_closed(storage, token);
                    continue 'user_loop;
                }

                if length <= (buffer.length() - buffer.cursor()) as u64 {
                    let mut errored = false;

                    if let Ok(bytes) = buffer.read_slice(length as usize) {
                        if packet.write_slice(bytes).is_err() {
                            errored = true;
                        }

                        packet.move_cursor_to_start();
                    } else {
                        errored = true;
                    }

                    if errored {
                        warn!("IP: {address} was disconnected due to error on packet length.");
                        set_client_as_closed(storage, token);
                        continue 'user_loop;
                    }

                    let socketid = SocketID { id: token, is_tls };

                    if handle_data(world, storage, &mut packet, entity, socketid).is_err() {
                        warn!("IP: {address} was disconnected due to invalid packets");
                        set_client_as_closed(storage, token);
                        continue 'user_loop;
                    }

                    count += 1
                } else {
                    let cursor = buffer.cursor() - 8;
                    buffer.move_cursor(cursor)?;
                    break;
                }

                if count == MAX_PROCESSED_PACKETS {
                    rerun.push(token);
                    break;
                }
            }

            let buffer_len = buffer.length() - buffer.cursor();

            if buffer.cursor() == buffer.length() {
                buffer.truncate(0)?;
                if buffer.capacity() > 500000 {
                    warn!(
                        "process_packets: buffer resize to 100000. Buffer Capacity: {}, Buffer len: {}",
                        buffer.capacity(),
                        buffer_len
                    );
                    buffer.resize(100000)?;
                }
            } else if buffer.capacity() > 500000 && buffer_len <= 100000 {
                warn!(
                    "process_packets: buffer resize to Buffer len. Buffer Capacity: {}, Buffer len: {}",
                    buffer.capacity(),
                    buffer_len
                );
                let mut replacement = ByteBuffer::with_capacity(buffer_len)?;
                replacement.write_slice(buffer.read_slice(buffer_len)?)?;
                replacement.move_cursor_to_start();
                *buffer = replacement;
            }
        };
    }

    let mut store = storage.recv_ids.borrow_mut();
    for token in rerun {
        store.insert(token);
    }

    Ok(())
}
