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
            poll_state: SocketPollState::Read,
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
        //We set it as None so we can fully control when to enable it again based on conditions.
        self.poll_state.set(SocketPollState::None);

        // Check if the Event has some readable Data from the Poll State.
        if event.is_readable() {
            self.read(world, storage);
        }

        // Check if the Event has some writable Data from the Poll State.
        if event.is_writable() {
            self.write(world);
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
        let socket = match world.get::<&mut Socket>(self.entity.0) {
            Ok(v) => v,
            Err(_) => {
                self.state = ClientState::Closing;
                return;
            }
        };

        // get the current pos so we can reset it back for reading.
        let mut buffer = socket.buffer.lock().unwrap();
        let pos = buffer.cursor();
        let _ = buffer.move_cursor_to_end();

        loop {
            let mut buf: [u8; 2048] = [0; 2048];
            match self.stream.read(&mut buf) {
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Ok(0) | Err(_) => {
                    self.state = ClientState::Closing;
                    return;
                }
                Ok(n) => {
                    if buffer.write_slice(&buf[0..n]).is_err() {
                        self.state = ClientState::Closing;
                        return;
                    }
                }
            }
        }

        // reset it back to the original pos so we can Read from it again.
        let _ = buffer.move_cursor(pos);

        if !buffer.is_empty() {
            storage.recv_ids.borrow_mut().insert(self.entity);
        } else {
            // we are not going to handle any reads so lets mark it back as read again so it can
            //continue to get packets.
            self.poll_state.add(SocketPollState::Read);
        }
    }

    pub fn write(&mut self, world: &mut hecs::World) {
        let mut count: usize = 0;

        //make sure the player exists if nto we have a socket closing
        if !world.contains(self.entity.0) {
            self.state = ClientState::Closing;
            return;
        }

        // lets only send 25 packets per socket each loop.
        while count < 25 {
            let mut packet = match self.sends.pop() {
                Some(packet) => packet,
                None => {
                    self.sends.shrink_to_fit();
                    return;
                }
            };

            match self.stream.write_all(packet.as_slice()) {
                Ok(()) => count += 1,
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

        if !self.sends.is_empty() {
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
    pub fn send(&mut self, poll: &mio::Poll, buf: ByteBuffer) {
        self.sends.insert(0, buf);

        self.poll_state.add(SocketPollState::Write);
        match self.reregister(poll) {
            Ok(_) => {}
            Err(_) => panic!("Socket did not reregister on Send write update."),
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
    server: &Server,
    socketid: usize,
    addr: String,
    world: &mut hecs::World,
    storage: &Storage,
) -> Option<Entity> {
    if server.clients.len() + 1 >= MAX_SOCKET_PLAYERS {
        return None;
    }

    storage.add_empty_player(world, socketid, addr).ok()
}

#[inline]
pub fn send_to(storage: &Storage, id: usize, buf: ByteBuffer) {
    if let Some(client) = storage.server.borrow().clients.get(&mio::Token(id)) {
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
        if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket.id)) {
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

            if let Some(client) = storage.server.borrow().clients.get(&mio::Token(socket.id)) {
                client
                    .borrow_mut()
                    .send(&storage.poll.borrow(), buf.clone());
            }
        }
    }
}
