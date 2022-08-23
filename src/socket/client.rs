use crate::{containers::Storage, gametypes::*, maps::*, players::Player, socket::*};
use mio::{net::TcpStream, Interest};
use std::io::{self, Read, Write};
use unwrap_helpers::*;

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
    pub playerid: usize,
    pub state: ClientState,
    pub sends: Vec<ByteBuffer>,
    pub poll_state: SocketPollState,
}

impl Client {
    #[inline]
    pub fn new(stream: TcpStream, token: mio::Token) -> Client {
        Client {
            stream,
            token,
            playerid: 0,
            state: ClientState::Open,
            sends: Vec::with_capacity(32),
            poll_state: SocketPollState::Read,
        }
    }

    pub fn process(&mut self, event: &mio::event::Event, world: &Storage) -> Result<()> {
        self.poll_state.set(SocketPollState::None);

        if event.is_readable() {
            self.read(world);
        }

        if event.is_writable() {
            self.write(world);
        }

        match self.state {
            ClientState::Closing => self.close_socket(world)?,
            _ => self.reregister(&world.poll.borrow_mut())?,
        }

        Ok(())
    }

    #[inline]
    pub fn set_to_closing(&mut self, world: &Storage) {
        self.state = ClientState::Closing;
        self.poll_state.add(SocketPollState::Write);
        self.reregister(&world.poll.borrow_mut()).unwrap();
    }

    #[inline]
    pub fn close_socket(&mut self, world: &Storage) -> Result<()> {
        match self.state {
            ClientState::Closed => Ok(()),
            _ => {
                self.stream.shutdown(std::net::Shutdown::Both)?;
                self.state = ClientState::Closed;
                disconnect(self.playerid, world);
                Ok(())
            }
        }
    }

    pub fn read(&mut self, world: &Storage) {
        if let Some(player) = world.players.borrow().get(self.playerid) {
            let pos = player.borrow().buffer.cursor();
            let _ = player
                .borrow_mut()
                .buffer
                .move_cursor(player.borrow().buffer.length());

            loop {
                let mut buf: [u8; 2048] = [0; 2048];
                match self.stream.read(&mut buf) {
                    Ok(0) => {
                        self.state = ClientState::Closing;
                        return;
                    }
                    Ok(n) => {
                        if player.borrow_mut().buffer.write_slice(&buf[0..n]).is_err() {
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

            let _ = player.borrow_mut().buffer.move_cursor(pos);

            if !player.borrow().buffer.is_empty()
                && !world.recv_ids.borrow().contains(&self.playerid)
            {
                world.recv_ids.borrow_mut().insert(self.playerid);
            }
        } else {
            self.poll_state.add(SocketPollState::Read);
        }
    }

    pub fn write(&mut self, world: &Storage) {
        let mut count: usize = 0;

        if let Some(_player) = world.players.borrow().get(self.playerid) {
            loop {
                let mut buffer = match self.sends.pop() {
                    Some(buffer) => buffer,
                    None => {
                        self.sends.shrink_to_fit();
                        return;
                    }
                };

                match self.stream.write(buffer.as_array()) {
                    Ok(n) => {
                        if n == 0 {
                            self.state = ClientState::Closing;
                            return;
                        }

                        count += 1;

                        if count >= 25 {
                            if !self.sends.is_empty() {
                                self.poll_state.add(SocketPollState::Write);
                            }

                            return;
                        }
                    }
                    Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(ref err) if err.kind() == io::ErrorKind::Interrupted => continue,
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
    pub fn send(&mut self, poll: &mio::Poll, buf: &ByteBuffer) {
        self.sends.insert(0, buf.clone());
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
pub fn disconnect(playerid: usize, world: &Storage) {
    if let Some(player) = world.remove_player(playerid) {
        if let Some(map) = world.map_data.get(&player.e.pos.map) {
            map.borrow_mut().remove_player(world, playerid);
            //todo Add save for player world here.
            //todo Add Update Players on map here.
        }
    }
}

#[inline]
pub fn accept_connection(socketid: usize, addr: String, world: &Storage) -> Option<usize> {
    if world.players.borrow().len() + 1 >= MAX_PLAYERS {
        return None;
    }

    let mut player = Player::new();
    player.socket_id = socketid;
    player.addr = addr;
    let id = world.add_player(player);
    Some(id)
}

#[inline]
pub fn send_to(world: &Storage, id: usize, buf: &ByteBuffer) {
    if let Some(mut client) = world.server.borrow().get_mut(mio::Token(id)) {
        client.send(&world.poll.borrow(), buf);
    }
}

#[inline]
pub fn send_to_all(world: &Storage, buf: &ByteBuffer) {
    for id in &*world.player_ids.borrow() {
        if let Some(player) = world.players.borrow().get(*id) {
            if let Some(mut client) = world
                .server
                .borrow()
                .get_mut(mio::Token(player.borrow().socket_id))
            {
                client.send(&world.poll.borrow(), buf);
            }
        }
    }
}

#[inline]
pub fn send_to_maps(
    world: &Storage,
    position: MapPosition,
    buf: &ByteBuffer,
    avoidindex: Option<usize>,
) {
    for m in get_surrounding(position, true) {
        let map = unwrap_continue!(world.map_data.get(&m));

        for id in &map.borrow().players {
            if avoidindex.map(|value| value == *id).unwrap_or(false) {
                continue;
            }

            if let Some(player) = world.players.borrow().get(*id) {
                if let Some(mut client) = world
                    .server
                    .borrow()
                    .get_mut(mio::Token(player.borrow().socket_id))
                {
                    client.send(&world.poll.borrow(), buf);
                }
            }
        }
    }
}
