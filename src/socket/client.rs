use crate::{containers::Storage, gametypes::*, maps::*, players::*, socket::*};
use mio::{net::TcpStream, Interest};
use std::io::{self, Read, Write};

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
        let self_size = *self as usize;
        let size = state as usize;

        if size == 0 || self_size == size {
            *self = state;
            return;
        }

        if self_size == 3 {
            return;
        }

        match self_size + size {
            1 => *self = SocketPollState::Read,
            2 => *self = SocketPollState::Write,
            3 => *self = SocketPollState::ReadWrite,
            _ => *self = SocketPollState::None,
        }
    }

    #[inline]
    pub fn set(&mut self, state: SocketPollState) {
        *self = state;
    }

    #[inline]
    pub fn remove(&mut self, state: SocketPollState) {
        let self_size = *self as usize;
        let size = state as usize;

        if size == 3 || self_size == size {
            *self = SocketPollState::None;
            return;
        }

        match self_size - size {
            1 => *self = SocketPollState::Read,
            2 => *self = SocketPollState::Write,
            _ => *self = SocketPollState::None,
        }
    }
}

#[derive(Debug)]
pub struct Client {
    pub stream: TcpStream,
    pub token: mio::Token,
    pub entity: Entity,
    pub state: ClientState,
    pub sends: Vec<ByteBuffer>,
    pub poll_state: SocketPollState,
    // used for sending encrypted Data.
    pub tls: rustls::ServerConnection,
    // If we are sending and Expecting TLS packets in return.
    // set to false when you 100% know the client should
    // Only send unencrypted Data.
    pub is_tls: bool,
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
            sends: Vec::with_capacity(32),
            poll_state: SocketPollState::ReadWrite,
            tls,
            is_tls: true,
        }
    }

    pub fn process(
        &mut self,
        event: &mio::event::Event,
        world: &mut hecs::World,
        storage: &Storage,
    ) -> Result<()> {
        self.poll_state.set(SocketPollState::None);

        if event.is_readable() {
            self.read_tls(world, storage);
        }

        if event.is_writable() {
            self.write_tls(world);
        }

        match self.state {
            ClientState::Closing => self.close_socket(world, storage)?,
            _ => self.reregister(&storage.poll.borrow_mut())?,
        }

        Ok(())
    }

    #[inline]
    pub fn set_to_closing(&mut self, storage: &Storage) {
        self.state = ClientState::Closing;
        self.poll_state.add(SocketPollState::Write);
        self.reregister(&storage.poll.borrow_mut()).unwrap();
    }

    #[inline]
    pub fn close_socket(&mut self, world: &mut hecs::World, storage: &Storage) -> Result<()> {
        match self.state {
            ClientState::Closed => Ok(()),
            _ => {
                self.stream.shutdown(std::net::Shutdown::Both)?;
                self.state = ClientState::Closed;
                disconnect(self.entity, world, storage);
                Ok(())
            }
        }
    }

    pub fn read(&mut self, world: &mut hecs::World, storage: &Storage) {
        if let Ok(data) = world.entity(self.entity.0) {
            let mut socket = data.get::<&mut Socket>().expect("Could not find Socket");
            let pos = socket.buffer.cursor();
            let _ = socket.buffer.move_cursor_to_end();

            loop {
                let mut buf: [u8; 2048] = [0; 2048];
                match self.stream.read(&mut buf) {
                    Ok(0) => {
                        self.state = ClientState::Closing;
                        return;
                    }
                    Ok(n) => {
                        if socket.buffer.write_slice(&buf[0..n]).is_err() {
                            self.state = ClientState::Closing;
                            return;
                        }
                    }
                    Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                    Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => {
                        self.state = ClientState::Closing;
                        return;
                    }
                }
            }

            let _ = socket.buffer.move_cursor(pos);

            if !socket.buffer.is_empty() && !storage.recv_ids.borrow().contains(&self.entity) {
                storage.recv_ids.borrow_mut().insert(self.entity);
            }
        } else {
            self.poll_state.add(SocketPollState::Read);
        }
    }

    pub fn read_tls(&mut self, world: &mut hecs::World, storage: &Storage) {
        match self.tls.read_tls(&mut self.stream) {
            Err(err) => {
                if let io::ErrorKind::WouldBlock = err.kind() {
                    return;
                }

                println!("read error {:?}", err);
                self.state = ClientState::Closing;
                return;
            }
            Ok(0) => {
                println!("eof");
                self.state = ClientState::Closing;
                return;
            }
            Ok(_) => {}
        };

        // Process newly-received TLS messages.
        match self.tls.process_new_packets() {
            Ok(io_state) => {
                if let Ok(data) = world.entity(self.entity.0) {
                    let mut socket = data.get::<&mut Socket>().expect("Could not find Socket");
                    let pos = socket.buffer.cursor();
                    let _ = socket.buffer.move_cursor_to_end();

                    loop {
                        if io_state.plaintext_bytes_to_read() > 0 {
                            let mut buf = vec![0u8; io_state.plaintext_bytes_to_read()];
                            match self.tls.reader().read_exact(&mut buf) {
                                Ok(()) => {
                                    if socket.buffer.write_slice(&buf[..]).is_err() {
                                        self.state = ClientState::Closing;
                                        return;
                                    }
                                }
                                Err(_) => {
                                    self.state = ClientState::Closing;
                                    return;
                                }
                            }
                        } else {
                            break;
                        }
                    }

                    let _ = socket.buffer.move_cursor(pos);

                    if !socket.buffer.is_empty()
                        && !storage.recv_ids.borrow().contains(&self.entity)
                    {
                        storage.recv_ids.borrow_mut().insert(self.entity);
                    }
                } else {
                    self.poll_state.add(SocketPollState::Read);
                }
            }
            Err(err) => {
                println!("cannot process packet: {:?}", err);

                // last gasp write to send any alerts
                let rc = self.tls.write_tls(&mut self.stream);
                if rc.is_err() {
                    println!("write failed {:?}", rc);
                }

                self.state = ClientState::Closing;
            }
        }
    }

    pub fn write(&mut self, world: &mut hecs::World) {
        let mut count: usize = 0;

        //make sure the player exists before we send anything.
        if world.contains(self.entity.0) {
            loop {
                let mut packet = match self.sends.pop() {
                    Some(packet) => packet,
                    None => {
                        self.sends.shrink_to_fit();
                        return;
                    }
                };

                match self.stream.write_all(packet.as_slice()) {
                    Ok(()) => {
                        count += 1;

                        if count >= 25 {
                            if !self.sends.is_empty() {
                                self.poll_state.add(SocketPollState::Write);
                            }

                            return;
                        }
                    }
                    Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                        //Operation would block so we insert it back in to try again later.
                        self.sends.push(packet);
                        return;
                    }
                    Err(_) => {
                        self.state = ClientState::Closing;
                        return;
                    }
                }
            }
        }
    }

    pub fn write_tls(&mut self, world: &mut hecs::World) {
        let mut count: usize = 0;

        //make sure the player exists before we send anything.
        if world.contains(self.entity.0) {
            loop {
                let mut packet = match self.sends.pop() {
                    Some(packet) => packet,
                    None => {
                        self.sends.shrink_to_fit();
                        return;
                    }
                };

                match self.tls.writer().write_all(packet.as_slice()) {
                    Ok(()) => {
                        count += 1;

                        if let Err(e) = self.tls.write_tls(&mut self.stream) {
                            println!("TLS write error: {}", e);
                        }

                        if count >= 25 {
                            if !self.sends.is_empty() {
                                self.poll_state.add(SocketPollState::Write);
                            }

                            return;
                        }
                    }
                    Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                        //Operation would block so we insert it back in to try again later.
                        self.sends.push(packet);
                        return;
                    }
                    Err(_) => {
                        self.state = ClientState::Closing;
                        return;
                    }
                }
            }
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
    pub fn send(&mut self, poll: &mio::Poll, buf: ByteBuffer) {
        self.sends.insert(0, buf);
        let state = self.poll_state;

        match state {
            SocketPollState::Write => {}
            SocketPollState::ReadWrite => {}
            _ => {
                self.poll_state.add(SocketPollState::Write);
                match self.reregister(poll) {
                    Ok(_) => {}
                    Err(_) => panic!("Socket did not reregister on Send write update."),
                }
            }
        }
    }
}

#[inline]
pub fn disconnect(playerid: Entity, world: &mut hecs::World, storage: &Storage) {
    if let Some((_socket, position)) = storage.remove_player(world, playerid) {
        if let Some(map) = storage.maps.get(&position.map) {
            map.borrow_mut().remove_player(storage, playerid);
            //todo Add save for player world here.
            //todo Add Update Players on map here.
        }
    }
}

#[inline]
pub fn accept_connection(
    socketid: usize,
    addr: String,
    world: &mut hecs::World,
    storage: &Storage,
) -> Option<Entity> {
    if storage.player_ids.borrow().len() + 1 >= MAX_SOCKET_PLAYERS {
        return None;
    }

    storage.add_empty_player(world, socketid, addr).ok()
}

#[inline]
pub fn send_to(storage: &Storage, id: usize, buf: ByteBuffer) {
    if let Some(client) = storage
        .server
        .borrow()
        .clients
        .borrow()
        .get(&mio::Token(id))
    {
        client.borrow_mut().send(&storage.poll.borrow(), buf);
    }
}

#[inline]
pub fn send_to_all(world: &hecs::World, storage: &Storage, buf: ByteBuffer) {
    for (_entity, (_, socket)) in world
        .query::<((&WorldEntityType, &OnlineType), &Socket)>()
        .iter()
        .filter(|(_entity, ((worldentitytype, onlinetype), _))| {
            **worldentitytype == WorldEntityType::Player && **onlinetype == OnlineType::Online
        })
    {
        if let Some(client) = storage
            .server
            .borrow()
            .clients
            .borrow()
            .get(&mio::Token(socket.id))
        {
            client
                .borrow_mut()
                .send(&storage.poll.borrow(), buf.clone());
        }
    }
}

#[inline]
pub fn send_to_maps(
    world: &hecs::World,
    storage: &Storage,
    position: MapPosition,
    buf: ByteBuffer,
    avoidindex: Option<Entity>,
) {
    for m in get_surrounding(position, true) {
        let map = match storage.maps.get(&m) {
            Some(map) => map,
            None => continue,
        }
        .borrow();

        for (entity, (_, socket)) in world
            .query::<((&WorldEntityType, &OnlineType, &Position), &Socket)>()
            .iter()
            .filter(|(_entity, ((worldentitytype, onlinetype, pos), _))| {
                **worldentitytype == WorldEntityType::Player
                    && **onlinetype == OnlineType::Online
                    && pos.map == map.position
            })
        {
            // ToDo: Avoid index
            if avoidindex
                .map(|value| value == Entity(entity))
                .unwrap_or(false)
            {
                continue;
            }

            if let Some(client) = storage
                .server
                .borrow()
                .clients
                .borrow()
                .get(&mio::Token(socket.id))
            {
                client
                    .borrow_mut()
                    .send(&storage.poll.borrow(), buf.clone());
            }
        }
    }
}
