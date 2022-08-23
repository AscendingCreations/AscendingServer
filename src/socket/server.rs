use crate::{
    containers::{HashMap, Storage},
    gametypes::Result,
    socket::{accept_connection, Client, ClientState},
};
use mio::{net::TcpListener, Events, Poll};
use std::{cell::RefCell, collections::VecDeque, io, time::Duration};
use unwrap_helpers::*;

pub const SERVER: mio::Token = mio::Token(0);

pub struct Server {
    pub listener: TcpListener,
    pub clients: HashMap<mio::Token, RefCell<Client>>,
    pub tokens: VecDeque<mio::Token>,
}

impl Server {
    #[inline]
    pub fn new(poll: &mut Poll, addr: &str, max: usize) -> Result<Server> {
        /* Create a bag of unique tokens. */
        let mut tokens = VecDeque::new();

        for i in 1..max {
            tokens.push_back(mio::Token(i));
        }

        /* Set up the TCP listener. */
        let addr = addr.parse()?;
        let mut listener = TcpListener::bind(addr)?;

        poll.registry()
            .register(&mut listener, SERVER, mio::Interest::READABLE)?;

        Ok(Server {
            listener,
            clients: HashMap::default(),
            tokens,
        })
    }

    pub fn accept(&mut self, world: &Storage) -> Result<()> {
        /* Wait for a new connection to accept and try to grab a token from the bag. */
        loop {
            let (stream, addr) = match self.listener.accept() {
                Ok((stream, addr)) => (stream, addr),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            };

            let token = self.tokens.pop_front();

            if let Some(token) = token {
                /* We got a unique token, now let's register the new connection. */
                let mut client = Client::new(stream, token);
                client.register(&world.poll.borrow_mut())?;

                let id =
                    unwrap_or_return!(accept_connection(token.0, addr.to_string(), world), || {
                        drop(client.stream);
                        Ok(())
                    });

                client.playerid = id;
                self.clients.insert(token, RefCell::new(client));
            } else {
                drop(stream);
            }
        }
        Ok(())
    }

    #[inline]
    pub fn get_mut(&self, token: mio::Token) -> Option<std::cell::RefMut<Client>> {
        /* Look up the connection for the given token. */
        if let Some(client) = self.clients.get(&token) {
            return Some(client.borrow_mut());
        }

        None
    }

    #[inline]
    pub fn get(&self, token: mio::Token) -> Option<std::cell::Ref<Client>> {
        /* Look up the connection for the given token. */
        if let Some(client) = self.clients.get(&token) {
            return Some(client.borrow());
        }

        None
    }

    #[inline]
    pub fn remove(&mut self, token: mio::Token) {
        /* If the token is valid, let's remove the connection and add the token back to the bag. */
        if self.clients.contains_key(&token) {
            self.clients.remove(&token);
            self.tokens.push_front(token);
        }
    }
}

pub fn poll_events(world: &Storage) -> Result<()> {
    let mut events = Events::with_capacity(1024);

    world
        .poll
        .borrow_mut()
        .poll(&mut events, Some(Duration::from_millis(100)))?;
    for event in events.iter() {
        match event.token() {
            SERVER => {
                world.server.borrow_mut().accept(world)?;
                world.poll.borrow_mut().registry().reregister(
                    &mut world.server.borrow_mut().listener,
                    SERVER,
                    mio::Interest::READABLE,
                )?;
            }
            token => {
                if let Some(mut a) = world.server.borrow_mut().get_mut(token) {
                    a.process(event, world)?;

                    if a.state == ClientState::Closed {
                        world.server.borrow_mut().remove(token);
                    }
                }
            }
        }
    }

    Ok(())
}
