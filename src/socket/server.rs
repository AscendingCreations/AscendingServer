use crate::{
    containers::{Storage, World},
    gametypes::Result,
    socket::{Client, ClientState},
};
use log::{trace, warn};
use mio::{Events, Poll, net::TcpListener};
use slab::Slab;
use std::{cell::RefCell, io, sync::Arc, time::Duration};

pub const SERVER: mio::Token = mio::Token(0);
pub const TLS_SERVER: mio::Token = mio::Token(1);
/// Needed Offset for the Token() to aovid using the same as the listeners.
pub const CLIENT_OFFSET: usize = 2;

pub struct Server {
    /// Listeners for None TLS Connections.
    pub listener: TcpListener,
    /// Listeners for TLS Connections.
    pub tls_listener: TcpListener,
    /// Each seperate Socket and its inner Data.
    pub clients: Slab<RefCell<Client>>,
    /// Number of connections allowed generally is max_allowed_players * 2
    pub max_allowed_connections: usize,
    /// Servers TLS encryption Config.
    pub tls_config: Arc<rustls::ServerConfig>,
}

impl Server {
    #[inline]
    pub fn new(
        poll: &mut Poll,
        addr: &str,
        tls_addr: &str,
        max_allowed_players: usize,
        cfg: Arc<rustls::ServerConfig>,
    ) -> Result<Server> {
        //We are handling x2 due to TLS + NonTLS socket per player. So we limit it to Players not actual connections.
        let max_allowed_connections = max_allowed_players * 2;
        /* Create a bag of unique tokens. */
        let clients = Slab::with_capacity(max_allowed_connections);

        /* Set up the TCP listener. */
        let addr = addr.parse()?;
        let mut listener = TcpListener::bind(addr)?;

        let tls_addr = tls_addr.parse()?;
        let mut tls_listener = TcpListener::bind(tls_addr)?;

        poll.registry()
            .register(&mut listener, SERVER, mio::Interest::READABLE)?;
        poll.registry()
            .register(&mut tls_listener, TLS_SERVER, mio::Interest::READABLE)?;

        Ok(Server {
            listener,
            tls_listener,
            clients,
            max_allowed_connections,
            tls_config: cfg,
        })
    }

    pub fn accept(&mut self, storage: &Storage, is_tls: bool) -> Result<()> {
        /* Wait for a new connection to accept and try to grab a token from the bag. */
        loop {
            let (stream, addr) = match if is_tls {
                self.tls_listener.accept()
            } else {
                self.listener.accept()
            } {
                Ok((stream, addr)) => (stream, addr),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    trace!("listener.accept error: {e}");
                    return Err(e.into());
                }
            };

            if !is_tls {
                stream.set_nodelay(true)?;
            }

            if self.clients.len() + 1 >= self.max_allowed_connections {
                warn!("Maxed Allowed Connections Reached. Dropping Stream.");
                drop(stream);
                return Ok(());
            }

            // Get the next Key before its used so we can create a client with it.
            let key = self.clients.vacant_key();

            let tls_conn = if is_tls {
                Some(rustls::ServerConnection::new(Arc::clone(&self.tls_config))?)
            } else {
                None
            };

            // Lets make the Client to handle hwo we send packets.
            let mut client = Client::new(
                stream,
                mio::Token(key + CLIENT_OFFSET),
                tls_conn,
                addr.to_string(),
            )?;

            client.poll_state.add(crate::socket::PollState::Write);
            //Register the Poll to the client for recv and Sending
            client.register(&storage.poll.borrow_mut())?;

            // insert client into handled list.
            self.clients.insert(RefCell::new(client));
        }
        Ok(())
    }

    #[inline]
    pub fn remove(&mut self, token: mio::Token) {
        let key = token.0 - CLIENT_OFFSET;

        if self.clients.contains(key) {
            self.clients.remove(key);
        }
    }
}

thread_local! {
    static EVENTS: RefCell<Events> = RefCell::new(Events::with_capacity(1024));
}

pub fn poll_events(world: &mut World, storage: &Storage) -> Result<()> {
    EVENTS.with_borrow_mut(|events| {
        storage
            .poll
            .borrow_mut()
            .poll(events, Some(Duration::from_millis(0)))
    })?;

    EVENTS.with_borrow_mut(|processing| {
        for event in processing.iter() {
            match event.token() {
                SERVER => {
                    storage.server.borrow_mut().accept(storage, false)?;
                    storage.poll.borrow_mut().registry().reregister(
                        &mut storage.server.borrow_mut().listener,
                        SERVER,
                        mio::Interest::READABLE,
                    )?;
                }
                TLS_SERVER => {
                    storage.server.borrow_mut().accept(storage, true)?;
                    storage.poll.borrow_mut().registry().reregister(
                        &mut storage.server.borrow_mut().tls_listener,
                        TLS_SERVER,
                        mio::Interest::READABLE,
                    )?;
                }
                token => {
                    let mut server = storage.server.borrow_mut();
                    let key = token.0 - CLIENT_OFFSET;
                    let state = if let Some(a) = server.clients.get(key) {
                        a.borrow_mut().process(event, world, storage)?;
                        a.borrow().state
                    } else {
                        trace!("a token no longer exists within clients.");
                        ClientState::Closed
                    };

                    if state == ClientState::Closed {
                        server.remove(token);
                    };
                }
            }
        }

        Ok(())
    })
}
