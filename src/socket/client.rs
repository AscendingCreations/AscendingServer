use crate::{
    containers::{GameStore, GameWorld},
    gametypes::*,
    handle_data,
    maps::*,
    players::*,
    socket::*,
    tasks::{unload_entity_packet, DataTaskToken},
};
use log::{error, info, trace, warn};
use mio::{net::TcpStream, Interest};
use mmap_bytey::BUFFER_SIZE;
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    sync::Arc,
};
use tokio::sync::RwLock;

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
    pub entity: Entity,
    pub state: ClientState,
    pub sends: VecDeque<MByteBuffer>,
    pub tls_sends: VecDeque<MByteBuffer>,
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
            poll_state: SocketPollState::ReadWrite,
            tls,
            encrypt_state: EncryptionState::ReadWrite,
        }
    }

    pub async fn process(
        &mut self,
        event: &mio::event::Event,
        world: &GameWorld,
        storage: &GameStore,
    ) -> Result<()> {
        //We set it as None so we can fully control when to enable it again based on conditions.
        self.poll_state.set(SocketPollState::Read);

        // Check if the Event has some readable Data from the Poll State.
        if event.is_readable() {
            if matches!(self.encrypt_state, EncryptionState::ReadWrite) {
                self.tls_read(storage).await?;
            } else {
                self.read(storage).await?;
            }
        }

        // Check if the Event has some writable Data from the Poll State.
        if event.is_writable() {
            if matches!(
                self.encrypt_state,
                EncryptionState::WriteTransfering | EncryptionState::ReadWrite
            ) {
                self.tls_write(world).await;
            } else {
                self.write(world).await;
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
            ClientState::Closing => self.close_socket(world, storage).await?,
            _ => self.reregister(&*storage.poll.read().await)?,
        }

        Ok(())
    }

    #[inline]
    pub fn deregister(&mut self, poll: &mio::Poll) -> Result<()> {
        Ok(poll.registry().deregister(&mut self.stream)?)
    }

    #[inline]
    pub async fn set_to_closing(&mut self, storage: &GameStore) -> Result<()> {
        self.state = ClientState::Closing;
        //self.poll_state.add(SocketPollState::Write);
        self.reregister(&*storage.poll.read().await)
    }

    #[inline]
    pub async fn close_socket(&mut self, world: &GameWorld, storage: &GameStore) -> Result<()> {
        match self.state {
            ClientState::Closed => Ok(()),
            _ => {
                //We dont care about errors here as they only occur when a socket is already disconnected by the client.
                info!("deregister socket");
                self.deregister(&*storage.poll.read().await)?;
                info!("shutdown stream");
                let _ = self.stream.shutdown(std::net::Shutdown::Both);
                self.state = ClientState::Closed;
                info!("disconnect");
                let ret = disconnect(self.entity, world, storage).await;
                info!("done close socket");
                ret
            }
        }
    }

    pub async fn tls_read(&mut self, storage: &GameStore) -> Result<()> {
        let arc_buffer = {
            let player_buffer_lock = storage.player_buffers.read().await;
            let buffer = match player_buffer_lock.get(&self.entity) {
                Some(v) => v.clone(),
                None => {
                    trace!("Player has no buffer");
                    self.state = ClientState::Closing;
                    return Ok(());
                }
            };
            buffer
        };
        let mut buffer = arc_buffer.write().await;

        // get the current pos so we can reset it back for reading.
        //let mut buffer = socket.buffer.lock().unwrap();

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
            storage.recv_ids.write().await.insert(self.entity);
        } else {
            // we are not going to handle any reads so lets mark it back as read again so it can
            //continue to get packets.
            self.poll_state.add(SocketPollState::Read);
        }

        Ok(())
    }

    pub async fn read(&mut self, storage: &GameStore) -> Result<()> {
        let arc_buffer = {
            let player_buffer_lock = storage.player_buffers.read().await;
            let buffer = match player_buffer_lock.get(&self.entity) {
                Some(v) => v.clone(),
                None => {
                    trace!("Player has no buffer");
                    self.state = ClientState::Closing;
                    return Ok(());
                }
            };
            buffer
        };
        let mut buffer = arc_buffer.write().await;

        // get the current pos so we can reset it back for reading.

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
            storage.recv_ids.write().await.insert(self.entity);
        } else {
            // we are not going to handle any reads so lets mark it back as read again so it can
            //continue to get packets.
            self.poll_state.add(SocketPollState::Read);
        }

        Ok(())
    }

    pub async fn write(&mut self, world: &GameWorld) {
        let mut count: usize = 0;

        //make sure the player exists if nto we have a socket closing
        if !world.contains(&self.entity).await {
            trace!(
                "write, world does not contain entity: {:?}. Closing socket.",
                self.entity.0
            );
            self.state = ClientState::Closing;
            return;
        }

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

    pub async fn tls_write(&mut self, world: &GameWorld) {
        //make sure the player exists if not we have a socket closing
        if !world.contains(&self.entity).await {
            trace!(
                "tls_write, world does not contain entity: {:?}. Closing socket.",
                self.entity.0
            );
            self.state = ClientState::Closing;
            return;
        }

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
pub async fn disconnect(playerid: Entity, world: &GameWorld, storage: &GameStore) -> Result<()> {
    info!("Left Game");
    left_game(world, storage, &playerid).await?;

    info!("remove player");
    let (socket, position) = storage.remove_player(world, playerid).await?;
    info!("remove buffers");
    storage.player_buffers.write().await.remove(&playerid);

    trace!("Players Disconnected IP: {} ", &socket.addr);
    if let Some(pos) = position {
        if let Some(map) = storage.maps.get(&pos.map) {
            {
                info!("map removes");
                let mut map_lock = map.write().await;
                map_lock.remove_player(storage, playerid).await;
                map_lock.remove_entity_from_grid(pos);
            }
            info!("datatask entity unload");
            DataTaskToken::EntityUnload(pos.map)
                .add_task(storage, unload_entity_packet(playerid)?)
                .await?;
        }
    }

    info!("done");
    Ok(())
}

#[inline]
pub async fn accept_connection(
    server: &RwLock<Server>,
    socketid: usize,
    addr: String,
    world: &GameWorld,
    storage: &GameStore,
) -> Option<Entity> {
    {
        let lock = server.read().await;

        if lock.clients.len() + 1 >= MAX_SOCKET_PLAYERS {
            warn!(
                "Server is full. has reached MAX_SOCKET_PLAYERS: {} ",
                MAX_SOCKET_PLAYERS
            );
            return None;
        }
    }

    storage.add_empty_player(world, socketid, addr).await.ok()
}

pub async fn set_encryption_status(
    storage: &GameStore,
    socket_id: usize,
    encryption_status: EncryptionState,
) {
    if let Some(client) = storage
        .server
        .read()
        .await
        .clients
        .get(&mio::Token(socket_id))
    {
        client.lock().await.encrypt_state = encryption_status;
    }
}

#[inline]
pub async fn send_to(storage: &GameStore, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage
        .server
        .read()
        .await
        .clients
        .get(&mio::Token(socket_id))
    {
        client.lock().await.send(&*storage.poll.read().await, buf)
    } else {
        Ok(())
    }
}

#[inline]
pub async fn tls_send_to(storage: &GameStore, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage
        .server
        .read()
        .await
        .clients
        .get(&mio::Token(socket_id))
    {
        client
            .lock()
            .await
            .tls_send(&*storage.poll.read().await, buf)
    } else {
        Ok(())
    }
}

#[inline]
pub async fn send_to_front(storage: &GameStore, socket_id: usize, buf: MByteBuffer) -> Result<()> {
    if let Some(client) = storage
        .server
        .read()
        .await
        .clients
        .get(&mio::Token(socket_id))
    {
        client
            .lock()
            .await
            .send_first(&*storage.poll.read().await, buf)
    } else {
        Ok(())
    }
}

#[inline]
pub async fn send_to_all(world: &GameWorld, storage: &GameStore, buf: MByteBuffer) -> Result<()> {
    let lock = world.read().await;
    for (_entity, (_, socket)) in lock
        .query::<((&WorldEntityType, &OnlineType), &Socket)>()
        .iter()
        .filter(|(_entity, ((worldentitytype, onlinetype), _))| {
            **worldentitytype == WorldEntityType::Player && **onlinetype == OnlineType::Online
        })
    {
        if let Some(client) = storage
            .server
            .read()
            .await
            .clients
            .get(&mio::Token(socket.id))
        {
            client
                .lock()
                .await
                .send(&*storage.poll.read().await, buf.try_clone()?)?;
        }
    }

    Ok(())
}

#[inline]
pub async fn send_to_maps(
    world: &GameWorld,
    storage: &GameStore,
    position: MapPosition,
    buf: MByteBuffer,
    avoidindex: Option<Entity>,
) -> Result<()> {
    for m in get_surrounding(position, true) {
        let map = match storage.maps.get(&m) {
            Some(map) => map,
            None => continue,
        };

        let players_on_map = {
            let map = map.read().await;
            let players = map.players.clone();
            players
        };

        for entity in &players_on_map {
            if avoidindex.map(|value| value == *entity).unwrap_or(false) {
                continue;
            }

            let lock = world.read().await;
            let mut query = lock.query_one::<(&OnlineType, &Socket)>(entity.0)?;

            if let Some((status, socket)) = query.get() {
                if *status == OnlineType::Online {
                    if let Some(client) = storage
                        .server
                        .read()
                        .await
                        .clients
                        .get(&mio::Token(socket.id))
                    {
                        client
                            .lock()
                            .await
                            .send(&*storage.poll.read().await, buf.try_clone()?)?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[inline]
pub async fn send_to_entities(
    world: &GameWorld,
    storage: &GameStore,
    entities: &[Entity],
    buf: MByteBuffer,
) -> Result<()> {
    for entity in entities {
        let lock = world.read().await;
        let mut query = lock.query_one::<(&OnlineType, &Socket)>(entity.0)?;

        if let Some((status, socket)) = query.get() {
            if *status == OnlineType::Online {
                if let Some(client) = storage
                    .server
                    .read()
                    .await
                    .clients
                    .get(&mio::Token(socket.id))
                {
                    client
                        .lock()
                        .await
                        .send(&*storage.poll.read().await, buf.try_clone()?)?;
                }
            }
        }
    }

    Ok(())
}

pub async fn get_length(
    storage: &GameStore,
    arc_buffer: &Arc<RwLock<ByteBuffer>>,
    id: usize,
) -> Result<Option<u64>> {
    let client = {
        let client = storage
            .server
            .read()
            .await
            .clients
            .get(&mio::Token(id))
            .cloned();
        client
    };

    let (buffer_length, cursor) = {
        let lock = arc_buffer.read().await;
        let length = lock.length();
        let cursor = lock.cursor();
        (length, cursor)
    };

    if buffer_length - cursor >= 8 {
        let length = arc_buffer.write().await.read::<u64>()?;

        if !(1..=8192).contains(&length) {
            if let Some(client) = client {
                trace!("Player was disconnected on get_length LENGTH: {:?}", length);
                client.lock().await.set_to_closing(storage).await?;
                return Ok(None);
            }
        }

        Ok(Some(length))
    } else {
        if let Some(client) = client {
            trace!("reregistering client");
            client
                .lock()
                .await
                .reregister(&*storage.poll.read().await)?;
            trace!("done reregistering client");
        }

        Ok(None)
    }
}

pub const MAX_PROCESSED_PACKETS: i32 = 25;

pub async fn process_packets(world: &GameWorld, storage: &GameStore) -> Result<()> {
    let mut packet = MByteBuffer::new()?;

    let recv_ids = {
        let mut lock = storage.recv_ids.write().await;
        let ids: Vec<Entity> = lock.drain(..).collect();
        ids
    };

    let mut reprocess = Vec::new();

    'user_loop: for entity in &recv_ids {
        let mut count = 0;

        let (socket_id, address) = {
            let lock = world.read().await;
            let socket = match lock.get::<&Socket>(entity.0) {
                Ok(s) => s,
                Err(e) => {
                    error!(
                        "world.get::<&Socket> error: {}. Entity: {:?}, did not get fully unloaded? recv_id buffer still existed.",
                        e, entity
                    );
                    continue 'user_loop;
                }
            };

            (socket.id, socket.addr.clone())
        };

        let arc_buffer = {
            let player_buffer_lock = storage.player_buffers.read().await;
            let buffer = match player_buffer_lock.get(entity) {
                Some(v) => v.clone(),
                None => {
                    trace!("Player has no buffer");
                    set_socket_to_closing(storage, socket_id).await;
                    continue 'user_loop;
                }
            };
            buffer
        };

        //let mut buffer = arc_buffer.write().await;

        'inner_loop: loop {
            packet.move_cursor_to_start();
            let length = {
                let len = match get_length(storage, &arc_buffer, socket_id).await {
                    Ok(s) => match s {
                        Some(n) => n,
                        None => {
                            break;
                        }
                    },
                    Err(e) => {
                        trace!("get_length. Error: {e}");
                        set_socket_to_closing(storage, socket_id).await;
                        continue 'user_loop;
                    }
                };
                len
            };

            if length == 0 {
                trace!(
                    "Length was Zero. Bad or malformed packet from IP: {}",
                    address
                );

                set_socket_to_closing(storage, socket_id).await;
                continue 'user_loop;
            }

            if length > BUFFER_SIZE as u64 {
                trace!(
                            "Length was {} greater than the max packet size of {}. Bad or malformed packet from IP: {}",
                            length,
                            address,
                            BUFFER_SIZE
                        );

                set_socket_to_closing(storage, socket_id).await;
                continue 'user_loop;
            }

            let (buffer_length, cursor) = {
                let lock = arc_buffer.read().await;
                let length = lock.length();
                let cursor = lock.cursor();
                (length, cursor)
            };

            if length <= (buffer_length - cursor) as u64 {
                let mut errored = false;

                if let Ok(bytes) = arc_buffer.write().await.read_slice(length as usize) {
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
                    set_socket_to_closing(storage, socket_id).await;
                    continue 'user_loop;
                }

                if handle_data(world, storage, &mut packet, entity)
                    .await
                    .is_err()
                {
                    warn!("IP: {} was disconnected due to invalid packets", address);
                    set_socket_to_closing(storage, socket_id).await;
                    continue 'user_loop;
                }

                count += 1
            } else {
                let cursor = arc_buffer.read().await.cursor() - 8;
                arc_buffer.write().await.move_cursor(cursor)?;

                break 'inner_loop;
            }

            if count == MAX_PROCESSED_PACKETS {
                break 'inner_loop;
            }
        }

        let cursor = arc_buffer.read().await.cursor();
        let buffer_len = arc_buffer.write().await.length() - cursor;

        if cursor == arc_buffer.read().await.length() {
            arc_buffer.write().await.truncate(0)?;

            if arc_buffer.read().await.capacity() > 500000 {
                warn!(
                    "process_packets: buffer resize to 100000. Buffer Capacity: {}, Buffer len: {}",
                    arc_buffer.read().await.capacity(),
                    buffer_len
                );
                arc_buffer.write().await.resize(100000)?;
            }
        } else if arc_buffer.read().await.capacity() > 500000 && buffer_len <= 100000 {
            warn!(
                "process_packets: buffer resize to Buffer len. Buffer Capacity: {}, Buffer len: {}",
                arc_buffer.read().await.capacity(),
                buffer_len
            );
            let mut replacement = ByteBuffer::with_capacity(buffer_len)?;

            {
                let mut buffer = arc_buffer.write().await;
                let read_slice = buffer.read_slice(buffer_len)?;
                replacement.write_slice(read_slice)?;
                replacement.move_cursor_to_start();
                *buffer = replacement;
            }
        }

        reprocess.push(entity);
    }

    {
        let mut lock = storage.recv_ids.write().await;

        for i in reprocess {
            lock.insert(*i);
        }
    }

    Ok(())
}

pub async fn set_socket_to_closing(storage: &GameStore, socket_id: usize) {
    if let Some(client) = storage
        .server
        .read()
        .await
        .clients
        .get(&mio::Token(socket_id))
    {
        if let Err(e) = client.lock().await.set_to_closing(storage).await {
            trace!("Player socket must not exit as could not be set to close. Error: {e}");
        }
    }
}
