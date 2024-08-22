use crate::{
    containers::{HashMap, Storage},
    gametypes::Result,
    socket::{accept_connection, Client, ClientState},
};
use hecs::World;
use log::{trace, warn};
use mio::{net::TcpListener, Events, Poll};
use std::{collections::VecDeque, io, sync::Arc, time::Duration};
use tokio::sync::Mutex;

pub const SERVER: mio::Token = mio::Token(0);

pub struct Server {
    pub listener: TcpListener,
    pub clients: HashMap<mio::Token, Mutex<Client>>,
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

    pub async fn accept(&mut self, world: &mut World, storage: &Storage) -> Result<()> {
        /* Wait for a new connection to accept and try to grab a token from the bag. */
        loop {
            let (stream, addr) = match self.listener.accept() {
                Ok((stream, addr)) => (stream, addr),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    trace!("listener.accept error: {}", e);
                    return Err(e.into());
                }
            };

            stream.set_nodelay(true)?;

            if let Some(token) = self.tokens.pop_front() {
                // Attempt to Create a Empty Player Entity.
                let entity = match accept_connection(
                    self,
                    token.0,
                    addr.to_string(),
                    world,
                    storage,
                )
                .await
                {
                    Some(e) => e,
                    None => {
                        drop(stream);
                        return Ok(());
                    }
                };

                let tls_conn = rustls::ServerConnection::new(Arc::clone(&self.tls_config))?;
                // Lets make the Client to handle hwo we send packets.
                let mut client = Client::new(stream, token, entity, tls_conn);
                //Register the Poll to the client for recv and Sending
                client.register(&*storage.poll.lock().await)?;

                // insert client into handled list.
                self.clients.insert(token, Mutex::new(client));
            } else {
                warn!("listener.accept No tokens left to give out.");
                drop(stream);
            }
        }
        Ok(())
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

pub async fn poll_events(world: &mut World, storage: &Storage) -> Result<()> {
    let mut events = Events::with_capacity(1024);

    storage
        .poll
        .lock()
        .await
        .poll(&mut events, Some(Duration::from_millis(0)))?;

    for event in events.iter() {
        match event.token() {
            SERVER => {
                storage.server.lock().await.accept(world, storage).await?;
                storage.poll.lock().await.registry().reregister(
                    &mut storage.server.lock().await.listener,
                    SERVER,
                    mio::Interest::READABLE,
                )?;
            }
            token => {
                let mut server = storage.server.lock().await;
                let state = if let Some(a) = server.clients.get(&token) {
                    let mut client = a.lock().await;
                    client.process(event, world, storage).await?;
                    client.state
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
}
