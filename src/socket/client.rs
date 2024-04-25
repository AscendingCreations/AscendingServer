use crate::{
    containers::Storage, gametypes::*, handle_data, maps::*, players::*, socket::*,
    tasks::DataTaskToken, PacketRouter,
};
use hecs::World;
use log::{error, warn};
use mio::{net::TcpStream, Interest};
use std::{
    cmp::max,
    collections::VecDeque,
    io::{self, Read, Write},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ClientState {
    Open,
    Closing,
    Closed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SocketPollState {
    None,
    Read,
    Write,
    ReadWrite,
}

impl SocketPollState {
    #[inline]
    pub fn add(&mut self, state: SocketPollState) {
        match (*self, state) {
            (SocketPollState::None, _) => *self = state,
            (SocketPollState::Read, SocketPollState::Write) => *self = SocketPollState::ReadWrite,
            (SocketPollState::Write, SocketPollState::Read) => *self = SocketPollState::ReadWrite,
            (_, _) => {}
        }
    }

    #[inline]
    pub fn set(&mut self, state: SocketPollState) {
        *self = state;
    }

    #[inline]
    pub fn remove(&mut self, state: SocketPollState) {
        match (*self, state) {
            (SocketPollState::Read, SocketPollState::Read) => *self = SocketPollState::None,
            (SocketPollState::Write, SocketPollState::Write) => *self = SocketPollState::None,
            (SocketPollState::ReadWrite, SocketPollState::Write) => *self = SocketPollState::Read,
            (SocketPollState::ReadWrite, SocketPollState::Read) => *self = SocketPollState::Write,
            (_, SocketPollState::ReadWrite) => *self = SocketPollState::None,
            (_, _) => {}
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EncryptionState {
    /// Send Unencrypted packets only.
    None,
    /// Send Encrypted for both read and write.
    ReadWrite,
    ///Migrating from encrypted to unencrypted after the last send.
    ///Read will start to read unencrypted traffic at this point.
    ///Only call this when we send the last nagotiation packet.
    WriteTransfering,
}

#[derive(Debug)]
pub struct Client {
    pub stream: TcpStream,
    pub token: mio::Token,
    pub entity: Entity,
    pub state: ClientState,
    pub sends: VecDeque<ByteBuffer>,
    pub tls_sends: VecDeque<ByteBuffer>,
    pub poll_state: SocketPollState,
    // used for sending encrypted Data.
    pub tls: rustls::ServerConnection,
    pub encrypt_state: EncryptionState,
}

impl Client {
    #[inline]
    pub fn new(
        stream: TcpStream,
        token: mio::Token,
        entity: Entity,
        tls: rustls::ServerConnection,
    ) -> Client {
        Client {
            stream,
            token,
            entity,
            state: ClientState::Open,
            sends: VecDeque::with_capacity(32),
            tls_sends: VecDeque::new(),
            poll_state: SocketPollState::Read,
            tls,
            encrypt_state: EncryptionState::ReadWrite,
        }
    }

    pub fn process(
        &mut self,
        event: &mio::event::Event,
        world: &mut World,
        storage: &Storage,
    ) -> Result<()> {
        //We set it as None so we can fully control when to enable it again based on conditions.
        self.poll_state.set(SocketPollState::Read);

        // Check if the Event has some readable Data from the Poll State.
        if event.is_readable() {
            if matches!(self.encrypt_state, EncryptionState::ReadWrite) {
                self.tls_read(world, storage)?;
            } else {
                self.read(world, storage)?;
            }
        }

        // Check if the Event has some writable Data from the Poll State.
        if event.is_writable() {
            if matches!(
                self.encrypt_state,
                EncryptionState::WriteTransfering | EncryptionState::ReadWrite
            ) {
                self.tls_write(world);
            } else {
                self.write(world);
            }
        }

        if self.encrypt_state == EncryptionState::WriteTransfering && self.tls_sends.is_empty() {
            self.tls_sends = VecDeque::new();
            self.encrypt_state = EncryptionState::None;
        } else {
            self.poll_state.add(SocketPollState::Write);
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

    #[inline]
    pub fn set_to_closing(&mut self, storage: &Storage) -> Result<()> {
        self.state = ClientState::Closing;
        self.poll_state.add(SocketPollState::Write);
        self.reregister(&storage.poll.borrow_mut())
    }

    #[inline]
    pub fn close_socket(&mut self, world: &mut World, storage: &Storage) -> Result<()> {
        match self.state {
            ClientState::Closed => Ok(()),
            _ => {
                //We dont care about errors here as they only occur when a socket is already disconnected by the client.
                self.deregister(&storage.poll.borrow_mut())?;
                let _ = self.stream.shutdown(std::net::Shutdown::Both);
                self.state = ClientState::Closed;
                disconnect(self.entity, world, storage)
            }
        }
    }

    pub fn tls_read(&mut self, world: &mut World, storage: &Storage) -> Result<()> {
        let socket = match world.get::<&mut Socket>(self.entity.0) {
            Ok(v) => v,
            Err(_) => {
                self.state = ClientState::Closing;
                return Ok(());
            }
        };

        // get the current pos so we can reset it back for reading.
        let mut buffer = socket.buffer.lock().unwrap();
        let pos = buffer.cursor();
        buffer.move_cursor_to_end();

        loop {
            match self.tls.read_tls(&mut self.stream) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(error) => {
                    log::error!("TLS read error: {:?}", error);
                    self.state = ClientState::Closing;
                    return Ok(());
                }
                Ok(0) => {
                    self.state = ClientState::Closing;
                    return Ok(());
                }
                Ok(_) => {}
            }

            let io_state = match self.tls.process_new_packets() {
                Ok(io_state) => io_state,
                Err(err) => {
                    log::error!("TLS error: {:?}", err);
                    self.state = ClientState::Closing;
                    return Ok(());
                }
            };

            if io_state.plaintext_bytes_to_read() > 0 {
                let mut buf = vec![0u8; io_state.plaintext_bytes_to_read()];
                if self.tls.reader().read_exact(&mut buf).is_err() {
                    self.state = ClientState::Closing;
                    return Ok(());
                }

                if buffer.write_slice(&buf).is_err() {
                    self.state = ClientState::Closing;
                    return Ok(());
                }
            }

            if io_state.peer_has_closed() {
                self.state = ClientState::Closing;
            }

            break;
        }

        // reset it back to the original pos so we can Read from it again.
        buffer.move_cursor(pos)?;

        if !buffer.is_empty() {
            storage.recv_ids.borrow_mut().insert(self.entity);
        } else {
            // we are not going to handle any reads so lets mark it back as read again so it can
            //continue to get packets.
            self.poll_state.add(SocketPollState::Read);
        }

        Ok(())
    }

    pub fn read(&mut self, world: &mut World, storage: &Storage) -> Result<()> {
        let socket = match world.get::<&mut Socket>(self.entity.0) {
            Ok(v) => v,
            Err(_) => {
                self.state = ClientState::Closing;
                return Ok(());
            }
        };

        // get the current pos so we can reset it back for reading.
        let mut buffer = socket.buffer.lock().unwrap();
        let pos = buffer.cursor();
        buffer.move_cursor_to_end();

        loop {
            let mut buf: [u8; 2048] = [0; 2048];
            match self.stream.read(&mut buf) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Ok(0) | Err(_) => {
                    self.state = ClientState::Closing;
                    return Ok(());
                }
                Ok(n) => {
                    if buffer.write_slice(&buf[0..n]).is_err() {
                        self.state = ClientState::Closing;
                        return Ok(());
                    }
                }
            }
        }

        // reset it back to the original pos so we can Read from it again.
        buffer.move_cursor(pos)?;

        if !buffer.is_empty() {
            storage.recv_ids.borrow_mut().insert(self.entity);
        } else {
            // we are not going to handle any reads so lets mark it back as read again so it can
            //continue to get packets.
            self.poll_state.add(SocketPollState::Read);
        }

        Ok(())
    }

    pub fn write(&mut self, world: &mut World) {
        let mut count: usize = 0;

        //make sure the player exists if nto we have a socket closing
        if !world.contains(self.entity.0) {
            self.state = ClientState::Closing;
            return;
        }

        // lets only send 25 packets per socket each loop.
        while count < 25 {
            let mut packet = match self.sends.pop_front() {
                Some(packet) => packet,
                None => {
                    if self.sends.capacity() > 50 {
                        self.sends.shrink_to_fit();
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
                Err(_) => {
                    self.state = ClientState::Closing;
                    return;
                }
            }
        }

        if !self.sends.is_empty() {
            self.poll_state.add(SocketPollState::Write);
        }
    }

    pub fn tls_write(&mut self, world: &mut World) {
        //make sure the player exists if not we have a socket closing
        if !world.contains(self.entity.0) {
            self.state = ClientState::Closing;
            return;
        }

        // lets only send 25 packets per socket each loop.
        loop {
            let mut packet = match self.tls_sends.pop_front() {
                Some(packet) => packet,
                None => {
                    if self.tls_sends.capacity() > 25 {
                        self.tls_sends.shrink_to_fit();
                    }
                    break;
                }
            };

            match self.tls.writer().write_all(packet.as_slice()) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.tls_sends.push_front(packet);
                    break;
                }
                Err(_) => {
                    self.state = ClientState::Closing;
                    return;
                }
                Ok(_) => {}
            }
        }

        loop {
            if self.tls.wants_write() {
                match self.tls.write_tls(&mut self.stream) {
                    Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(ref err) if err.kind() == io::ErrorKind::Interrupted => {
                        continue;
                    }
                    Err(_) => {
                        self.state = ClientState::Closing;
                        return;
                    }
                    Ok(_) => {}
                };
            } else {
                break;
            }
        }

        if !self.tls_sends.is_empty() {
            self.poll_state.add(SocketPollState::Write);
        }
    }

    #[inline]
    pub fn event_set(&mut self) -> Option<Interest> {
        match self.poll_state {
            SocketPollState::None => None,
            SocketPollState::Read => Some(Interest::READABLE),
            SocketPollState::Write => Some(Interest::WRITABLE),
            SocketPollState::ReadWrite => Some(Interest::READABLE.add(Interest::WRITABLE)),
        }
    }

    #[inline]
    pub fn register(&mut self, poll: &mio::Poll) -> Result<()> {
        if let Some(interest) = self.event_set() {
            poll.registry()
                .register(&mut self.stream, self.token, interest)?;
        }
        Ok(())
    }

    #[inline]
    pub fn reregister(&mut self, poll: &mio::Poll) -> Result<()> {
        if let Some(interest) = self.event_set() {
            poll.registry()
                .reregister(&mut self.stream, self.token, interest)?;
        }
        Ok(())
    }

    #[inline]
    pub fn send(&mut self, poll: &mio::Poll, buf: ByteBuffer) -> Result<()> {
        self.sends.push_back(buf);
        self.poll_state.add(SocketPollState::Write);
        self.reregister(poll)
    }

    #[inline]
    pub fn tls_send(&mut self, poll: &mio::Poll, buf: ByteBuffer) -> Result<()> {
        self.tls_sends.push_back(buf);
        self.poll_state.add(SocketPollState::Write);
        self.reregister(poll)
    }
}

#[inline]
pub fn disconnect(playerid: Entity, world: &mut World, storage: &Storage) -> Result<()> {
    left_game(world, storage, &playerid)?;

    let (socket, position) = storage.remove_player(world, playerid)?;

    println!("Players Disconnected IP: {} ", &socket.addr);
    if let Some(pos) = position {
        if let Some(map) = storage.maps.get(&pos.map) {
            map.borrow_mut().remove_player(storage, playerid);
            map.borrow_mut().remove_entity_from_grid(pos);
            DataTaskToken::EntityUnload(pos.map).add_task(storage, &(playerid))?;
        }
    }

    Ok(())
}

#[inline]
pub fn accept_connection(
    server: &Server,
    socketid: usize,
    addr: String,
    world: &mut World,
    storage: &Storage,
) -> Option<Entity> {
    if server.clients.len() + 1 >= MAX_SOCKET_PLAYERS {
        return None;
    }

    storage.add_empty_player(world, socketid, addr).ok()
}

pub fn set_encryption_status(
    storage: &Storage,
    socket_id: usize,
    encryption_status: EncryptionState,
) {
    if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket_id)) {
        client.borrow_mut().encrypt_state = encryption_status;
    }
}

#[inline]
pub fn send_to(storage: &Storage, socket_id: usize, buf: ByteBuffer) -> Result<()> {
    if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket_id)) {
        if client.borrow().encrypt_state != EncryptionState::None {
            client.borrow_mut().tls_send(&storage.poll.borrow(), buf)
        } else {
            client.borrow_mut().send(&storage.poll.borrow(), buf)
        }
    } else {
        Ok(())
    }
}

#[inline]
pub fn send_to_all(world: &mut World, storage: &Storage, buf: ByteBuffer) -> Result<()> {
    for (_entity, (_, socket)) in world
        .query::<((&WorldEntityType, &OnlineType), &Socket)>()
        .iter()
        .filter(|(_entity, ((worldentitytype, onlinetype), _))| {
            **worldentitytype == WorldEntityType::Player && **onlinetype == OnlineType::Online
        })
    {
        if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket.id)) {
            client
                .borrow_mut()
                .send(&storage.poll.borrow(), buf.clone())?;
        }
    }

    Ok(())
}

#[inline]
pub fn send_to_maps(
    world: &mut World,
    storage: &Storage,
    position: MapPosition,
    buf: ByteBuffer,
    avoidindex: Option<Entity>,
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

            let (status, socket) = world.query_one_mut::<(&OnlineType, &Socket)>(entity.0)?;

            if *status == OnlineType::Online {
                if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket.id)) {
                    client
                        .borrow_mut()
                        .send(&storage.poll.borrow(), buf.clone())?;
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
    entities: &[Entity],
    buf: ByteBuffer,
) -> Result<()> {
    for entity in entities {
        let (status, socket) = world.query_one_mut::<(&OnlineType, &Socket)>(entity.0)?;

        if *status == OnlineType::Online {
            if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket.id)) {
                client
                    .borrow_mut()
                    .send(&storage.poll.borrow(), buf.clone())?;
            }
        }
    }

    Ok(())
}

pub fn get_length(storage: &Storage, buffer: &mut ByteBuffer, id: usize) -> Result<Option<u64>> {
    if buffer.length() - buffer.cursor() >= 8 {
        let length = buffer.read::<u64>()?;

        if !(1..=8192).contains(&length) {
            if let Some(client) = storage.server.borrow().clients.get(&mio::Token(id)) {
                warn!("Player was disconnected on get_length LENGTH: {:?}", length);
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
    let mut rem_arr: Vec<Entity> = Vec::with_capacity(32);
    let mut close_arr: Vec<(Entity, usize)> = Vec::with_capacity(32);
    let max_packet_handle = max(1000 / (storage.recv_ids.borrow().len() + 1), 25);

    'user_loop: for entity in &*storage.recv_ids.borrow() {
        let mut count = 0;

        let (lock, socket_id, address) = {
            let socket = match world.get::<&Socket>(entity.0) {
                Ok(s) => s,
                Err(_) => {
                    error!(
                        "Entity: {:?}, did not get fully unloaded. recv_id buffer still existed.",
                        entity
                    );
                    rem_arr.push(*entity);
                    continue 'user_loop;
                }
            };

            (socket.buffer.clone(), socket.id, socket.addr.clone())
        };

        if let Ok(mut buffer) = lock.lock() {
            loop {
                let length = match get_length(storage, &mut buffer, socket_id)? {
                    Some(n) => n,
                    None => {
                        rem_arr.push(*entity);
                        break;
                    }
                };

                if length == 0 {
                    warn!(
                        "Length was Zero. Bad or malformed packet from IP: {}",
                        address
                    );

                    close_arr.push((*entity, socket_id));
                    continue 'user_loop;
                }

                if length <= (buffer.length() - buffer.cursor()) as u64 {
                    let mut buffer = match buffer.read_to_buffer(length as usize) {
                        Ok(n) => n,
                        Err(_) => {
                            warn!(
                                "IP: {} was disconnected due to error on packet length.",
                                address
                            );
                            close_arr.push((*entity, socket_id));
                            continue 'user_loop;
                        }
                    };

                    if handle_data(router, world, storage, &mut buffer, entity).is_err() {
                        warn!("IP: {} was disconnected due to invalid packets", address);
                        close_arr.push((*entity, socket_id));
                        continue 'user_loop;
                    }

                    count += 1
                } else {
                    let cursor = buffer.cursor() - 8;
                    buffer.move_cursor(cursor)?;

                    rem_arr.push(*entity);
                    break;
                }

                if count == max_packet_handle {
                    break;
                }
            }

            let buffer_len = buffer.length() - buffer.cursor();

            if buffer.cursor() == buffer.length() {
                buffer.truncate(0)?;
                if buffer.capacity() > 25000 {
                    buffer.resize(4096)?;
                }
            } else if buffer.capacity() > 25000 && buffer_len <= 10000 {
                let mut replacement = ByteBuffer::with_capacity(buffer_len)?;
                replacement.write_slice(buffer.read_slice(buffer_len)?)?;
                replacement.move_cursor_to_start();
                *buffer = replacement;
            }
        };
    }

    for i in rem_arr {
        storage.recv_ids.borrow_mut().swap_remove(&i);
    }

    for (entity, socket_id) in close_arr {
        storage.recv_ids.borrow_mut().swap_remove(&entity);
        if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket_id)) {
            client.borrow_mut().set_to_closing(storage)?;
        }
    }

    Ok(())
}
