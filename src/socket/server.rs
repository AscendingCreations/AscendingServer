use crate::{
    containers::{GameStore, GameWorld, HashMap},
    gametypes::Result,
    socket::{accept_connection, Client, ClientState},
};
use log::{trace, warn};
use mio::{net::TcpListener, Events, Poll};
use std::{collections::VecDeque, io, sync::Arc, time::Duration};
use tokio::sync::{Mutex, RwLock};

pub const SERVER: mio::Token = mio::Token(0);

pub struct Server {
    pub listener: TcpListener,
    pub clients: HashMap<mio::Token, Arc<Mutex<Client>>>,
    pub tokens: VecDeque<mio::Token>,
    pub tls_config: Arc<rustls::ServerConfig>,
}

impl Server {
    #[inline]
    pub fn new(
        poll: &mut Poll,
        addr: &str,
        max: usize,
        cfg: Arc<rustls::ServerConfig>,
    ) -> Result<Server> {
        /* Create a bag of unique tokens. */
        let mut tokens = VecDeque::with_capacity(max);

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
            tls_config: cfg,
        })
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

pub async fn accept(server: &RwLock<Server>, world: &GameWorld, storage: &GameStore) -> Result<()> {
    /* Wait for a new connection to accept and try to grab a token from the bag. */
    loop {
        let (stream, addr) = {
            let lock = server.read().await;
            let (stream, addr) = match lock.listener.accept() {
                Ok((stream, addr)) => (stream, addr),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    trace!("listener.accept error: {}", e);
                    return Err(e.into());
                }
            };
            (stream, addr)
        };

        stream.set_nodelay(true)?;

        let token = {
            let mut lock = server.write().await;
            let token = lock.tokens.pop_front();
            token
        };

        if let Some(token) = token {
            // Attempt to Create a Empty Player Entity.
            let entity =
                match accept_connection(server, token.0, addr.to_string(), world, storage).await {
                    Some(e) => e,
                    None => {
                        drop(stream);
                        return Ok(());
                    }
                };

            let tls_conn = {
                let lock = server.read().await;
                let tls_connection = rustls::ServerConnection::new(Arc::clone(&lock.tls_config))?;
                tls_connection
            };

            // Lets make the Client to handle hwo we send packets.
            let mut client = Client::new(stream, token, entity, tls_conn);
            //Register the Poll to the client for recv and Sending
            client.register(&*storage.poll.read().await)?;

            // insert client into handled list.

            server
                .write()
                .await
                .clients
                .insert(token, Arc::new(Mutex::new(client)));
        } else {
            warn!("listener.accept No tokens left to give out.");
            drop(stream);
        }
    }
    Ok(())
}

pub async fn poll_events(world: &GameWorld, storage: &GameStore) -> Result<()> {
    let mut events = Events::with_capacity(1024);

    match storage
        .poll
        .write()
        .await
        .poll(&mut events, Some(Duration::from_millis(0)))
    {
        Ok(()) => {}
        Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(()),
        Err(ref err) if err.kind() == io::ErrorKind::Interrupted => return Ok(()),
        Err(e) => {
            trace!("listener.accept error: {}", e);
            return Err(e.into());
        }
    };

    for event in events.iter() {
        match event.token() {
            SERVER => {
                accept(&storage.server, world, storage).await?;
                storage.poll.read().await.registry().reregister(
                    &mut storage.server.write().await.listener,
                    SERVER,
                    mio::Interest::READABLE,
                )?;
            }
            token => {
                let client = {
                    let server = storage.server.read().await;
                    let client = server.clients.get(&token).cloned();
                    client
                };

                let state = if let Some(a) = client {
                    let mut client = a.lock().await;
                    client.process(event, world, storage).await?;
                    client.state
                } else {
                    trace!("a token no longer exists within clients.");
                    ClientState::Closed
                };

                if state == ClientState::Closed {
                    let mut server = storage.server.write().await;
                    server.remove(token);
                };
            }
        }
    }

    Ok(())
}
