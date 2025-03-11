use crate::{
    PacketRouter,
    containers::{Entity, GlobalKey, Storage, World},
    gameloop::SocketID,
    gametypes::*,
    handle_data,
    maps::*,
    players::*,
    socket::*,
    tasks::{DataTaskToken, unload_entity_packet},
};
use log::{error, trace, warn};
use mio::{Interest, Token, net::TcpStream};
use mmap_bytey::BUFFER_SIZE;
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::{Arc, Mutex},
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

    pub fn contains(&mut self, state: SocketPollState) -> bool {
        ((*self == SocketPollState::Read || *self == SocketPollState::ReadWrite)
            && (state == SocketPollState::Read || state == SocketPollState::ReadWrite))
            || ((*self == SocketPollState::Write || *self == SocketPollState::ReadWrite)
                && (state == SocketPollState::Write || state == SocketPollState::ReadWrite))
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
    pub entity: Option<GlobalKey>,
    pub state: ClientState,
    pub sends: VecDeque<MByteBuffer>,
    pub tls_sends: VecDeque<MByteBuffer>,
    pub poll_state: SocketPollState,
    // used for sending encrypted Data.
    pub tls: rustls::ServerConnection,
    pub buffer: Arc<Mutex<ByteBuffer>>,
    pub encrypt_state: EncryptionState,
    pub addr: Arc<String>,
}

impl Client {
    #[inline]
    pub fn new(
        stream: TcpStream,
        token: mio::Token,
        tls: rustls::ServerConnection,
        addr: String,
    ) -> Result<Client> {
        Ok(Client {
            stream,
            token,
            entity: None,
            state: ClientState::Open,
            sends: VecDeque::with_capacity(32),
            tls_sends: VecDeque::new(),
            poll_state: SocketPollState::Read,
            tls,
            buffer: Arc::new(Mutex::new(ByteBuffer::with_capacity(8192)?)),
            encrypt_state: EncryptionState::ReadWrite,
            addr: Arc::new(addr),
        })
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
                self.tls_read(storage)?;
            } else {
                self.read(storage)?;
            }
        }

        // Check if the Event has some writable Data from the Poll State.
        if event.is_writable() {
            if matches!(
                self.encrypt_state,
                EncryptionState::WriteTransfering | EncryptionState::ReadWrite
            ) {
                self.tls_write();
            } else {
                self.write();
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
                if let Some(entity) = self.entity {
                    disconnect(entity, world, storage)?;
                }
                Ok(())
            }
        }
    }

    pub fn tls_read(&mut self, storage: &Storage) -> Result<()> {
        // get the current pos so we can reset it back for reading.
        let mut buffer = self.buffer.lock().unwrap();
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
                    error!("TLS read error: {:?}", error);
                    self.state = ClientState::Closing;
                    return Ok(());
                }
                Ok(0) => {
                    trace!("Client side socket closed");
                    self.state = ClientState::Closing;
                    return Ok(());
                }
                Ok(_) => {}
            }

            let io_state = match self.tls.process_new_packets() {
                Ok(io_state) => io_state,
                Err(err) => {
                    error!("TLS error: {:?}", err);
                    self.state = ClientState::Closing;
                    return Ok(());
                }
            };

            if io_state.plaintext_bytes_to_read() > 0 {
                let mut buf = vec![0u8; io_state.plaintext_bytes_to_read()];
                if let Err(e) = self.tls.reader().read_exact(&mut buf) {
                    trace!("TLS read error: {}", e);
                    self.state = ClientState::Closing;
                    return Ok(());
                }

                if let Err(e) = buffer.write_slice(&buf) {
                    trace!("TLS read error: {}", e);
                    self.state = ClientState::Closing;
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
            storage.recv_ids.borrow_mut().insert(self.token);
        } else {
            // we are not going to handle any reads so lets mark it back as read again so it can
            //continue to get packets.
            self.poll_state.add(SocketPollState::Read);
        }

        Ok(())
    }

    pub fn read(&mut self, storage: &Storage) -> Result<()> {
        // get the current pos so we can reset it back for reading.
        let mut buffer = self.buffer.lock().unwrap();
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
                    trace!("stream.read, error in socket read: {}", e);
                    closing = true;
                }
                Ok(n) => {
                    if let Err(e) = buffer.write_slice(&buf[0..n]) {
                        trace!("buffer.write_slice, error in socket read: {}", e);
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
            storage.recv_ids.borrow_mut().insert(self.token);
        } else {
            // we are not going to handle any reads so lets mark it back as read again so it can
            //continue to get packets.
            self.poll_state.add(SocketPollState::Read);
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
                    trace!("stream.write_all error in socket write: {}", e);
                    self.state = ClientState::Closing;
                    return;
                }
            }
        }

        if !self.sends.is_empty() {
            self.poll_state.add(SocketPollState::Write);
        }
    }

    pub fn tls_write(&mut self) {
        // lets only send 25 packets per socket each loop.
        loop {
            let mut packet = match self.tls_sends.pop_front() {
                Some(packet) => packet,
                None => {
                    if self.tls_sends.capacity() > 100 {
                        warn!(
                            "Socket TLSwrite: tls_sends Buffer Strink to 100, Current Capacity {}, Current len {}.",
                            self.tls_sends.capacity(),
                            self.tls_sends.len()
                        );
                        self.tls_sends.shrink_to(100);
                    }
                    break;
                }
            };

            match self.tls.writer().write_all(packet.as_slice()) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.tls_sends.push_front(packet);
                    break;
                }
                Err(e) => {
                    trace!("tls write, error in write_all: {}", e);
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
                    Err(e) => {
                        trace!("tls write, error in write_tls: {}", e);
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
    pub fn send(&mut self, poll: &mio::Poll, buf: MByteBuffer) -> Result<()> {
        self.sends.push_back(buf);
        self.add_write_state(poll)
    }

    #[inline]
    pub fn send_first(&mut self, poll: &mio::Poll, buf: MByteBuffer) -> Result<()> {
        self.sends.push_front(buf);
        self.add_write_state(poll)
    }

    #[inline]
    pub fn tls_send(&mut self, poll: &mio::Poll, buf: MByteBuffer) -> Result<()> {
        self.tls_sends.push_back(buf);
        self.add_write_state(poll)
    }

    #[inline]
    pub fn add_write_state(&mut self, poll: &mio::Poll) -> Result<()> {
        if !self.poll_state.contains(SocketPollState::Write) {
            self.poll_state.add(SocketPollState::Write);
            self.reregister(poll)?;
        }

        Ok(())
    }
}

#[inline]
pub fn disconnect(playerid: GlobalKey, world: &mut World, storage: &Storage) -> Result<()> {
    left_game(world, storage, playerid)?;

    let result = storage.remove_player(world, playerid)?;

    if let Some((socket, pos)) = result {
        trace!("Players Disconnected IP: {} ", &socket.addr);

        if let Some(map) = storage.maps.get(&pos.map) {
            map.borrow_mut().remove_player(storage, playerid);
            map.borrow_mut().remove_entity_from_grid(pos);
            DataTaskToken::EntityUnload(pos.map)
                .add_task(storage, unload_entity_packet(playerid)?)?;
        }
    }

    Ok(())
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
pub fn send_to(storage: &Storage, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket_id)) {
        client.borrow_mut().send(&storage.poll.borrow(), buf)
    } else {
        Ok(())
    }
}

#[inline]
pub fn tls_send_to(storage: &Storage, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket_id)) {
        client.borrow_mut().tls_send(&storage.poll.borrow(), buf)
    } else {
        Ok(())
    }
}

#[inline]
pub fn send_to_front(storage: &Storage, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket_id)) {
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

            if data.online_type == OnlineType::Online {
                if let Some(client) = storage.server.borrow().clients.get(&Token(data.socket.id)) {
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

                if data.online_type == OnlineType::Online {
                    if let Some(client) =
                        storage.server.borrow().clients.get(&Token(data.socket.id))
                    {
                        client
                            .borrow_mut()
                            .send(&storage.poll.borrow(), buf.try_clone()?)?;
                    }
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

            if data.online_type == OnlineType::Online {
                if let Some(client) = storage.server.borrow().clients.get(&Token(data.socket.id)) {
                    client
                        .borrow_mut()
                        .send(&storage.poll.borrow(), buf.try_clone()?)?;
                }
            }
        }
    }

    Ok(())
}

pub fn get_length(storage: &Storage, buffer: &mut ByteBuffer, token: Token) -> Result<Option<u64>> {
    if buffer.length() - buffer.cursor() >= 8 {
        let length = buffer.read::<u64>()?;

        if !(1..=8192).contains(&length) {
            if let Some(client) = storage.server.borrow().clients.get(&token) {
                trace!("Player was disconnected on get_length LENGTH: {:?}", length);
                client.borrow_mut().set_to_closing(storage)?;
                return Ok(None);
            }
        }

        Ok(Some(length))
    } else {
        if let Some(client) = storage.server.borrow().clients.get(&token) {
            client.borrow_mut().poll_state.add(SocketPollState::Read);
            client.borrow_mut().reregister(&storage.poll.borrow_mut())?;
        }

        Ok(None)
    }
}

pub const MAX_PROCESSED_PACKETS: i32 = 25;

pub fn process_packets(world: &mut World, storage: &Storage, router: &PacketRouter) -> Result<()> {
    let mut rem_arr: Vec<(Token, bool)> = Vec::with_capacity(64);
    let mut packet = MByteBuffer::new()?;

    'user_loop: for token in &*storage.recv_ids.borrow() {
        let mut count = 0;

        let (lock, entity, address) = {
            match storage.server.borrow().clients.get(token) {
                Some(v) => {
                    let brw_client = v.borrow();
                    (
                        brw_client.buffer.clone(),
                        brw_client.entity,
                        brw_client.addr.clone(),
                    )
                }
                None => {
                    error!("Socket was missing in server clients.");
                    rem_arr.push((*token, false));
                    continue 'user_loop;
                }
            }
        };

        if let Ok(mut buffer) = lock.lock() {
            loop {
                packet.move_cursor_to_start();
                let length = match get_length(storage, &mut buffer, *token)? {
                    Some(n) => n,
                    None => {
                        rem_arr.push((*token, false));
                        break;
                    }
                };

                if length == 0 {
                    trace!(
                        "Length was Zero. Bad or malformed packet from IP: {}",
                        address
                    );

                    rem_arr.push((*token, true));
                    continue 'user_loop;
                }

                if length > BUFFER_SIZE as u64 {
                    trace!(
                        "Length was {} greater than the max packet size of {}. Bad or malformed packet from IP: {}",
                        length, address, BUFFER_SIZE
                    );

                    rem_arr.push((*token, true));
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
                        warn!(
                            "IP: {} was disconnected due to error on packet length.",
                            address
                        );
                        rem_arr.push((*token, true));
                        continue 'user_loop;
                    }

                    let socketid = SocketID { id: *token };

                    if handle_data(router, world, storage, &mut packet, entity, socketid).is_err() {
                        warn!("IP: {} was disconnected due to invalid packets", address);
                        rem_arr.push((*token, true));
                        continue 'user_loop;
                    }

                    count += 1
                } else {
                    let cursor = buffer.cursor() - 8;
                    buffer.move_cursor(cursor)?;

                    rem_arr.push((*token, false));
                    break;
                }

                if count == MAX_PROCESSED_PACKETS {
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

    for (token, should_close) in rem_arr {
        storage.recv_ids.borrow_mut().swap_remove(&token);

        if should_close {
            if let Some(client) = storage.server.borrow().clients.get(&token) {
                client.borrow_mut().set_to_closing(storage)?;
            }
        }
    }

    Ok(())
}
