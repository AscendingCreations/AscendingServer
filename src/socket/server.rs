use crate::{
    containers::{GameStore, GameWorld, HashMap},
    gametypes::Result,
    socket::{accept_connection, Client, ClientState},
    tasks::{process_data_lists, process_tasks},
};
use log::{info, trace, warn};
use mio::{net::TcpListener, Events, Poll};
use std::{collections::VecDeque, io, sync::Arc, time::Duration};
use tokio::sync::{Mutex, RwLock};

use super::process_packets;

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
            storage.packet_process_ids.write().await.push_back(entity);
        } else {
            warn!("listener.accept No tokens left to give out.");
            drop(stream);
        }
    }
    Ok(())
}

pub async fn poll_events(world: GameWorld, storage: GameStore) -> Result<()> {
    let mut events = Events::with_capacity(1024);

    while !{
        let stop = *storage.disable_threads.read().await;
        stop
    } {
        if let Err(e) = storage
            .poll
            .write()
            .await
            .poll(&mut events, Some(Duration::from_millis(0)))
        {
            info!("oop error poll: {e}");
        }

        for event in events.iter() {
            match event.token() {
                SERVER => {
                    if let Err(e) = accept(&storage.server, &world, &storage).await {
                        info!("oop error accept: {e}");
                    }
                    if let Err(e) = storage.poll.read().await.registry().reregister(
                        &mut storage.server.write().await.listener,
                        SERVER,
                        mio::Interest::READABLE,
                    ) {
                        info!("oop error reregister: {e}");
                    }
                }
                token => {
                    let client = {
                        let server = storage.server.read().await;
                        let client = server.clients.get(&token).cloned();
                        client
                    };

                    let state = if let Some(a) = client {
                        let mut client = a.lock().await;
                        if let Err(e) = client.process(event, &world, &storage).await {
                            info!("oop error client process: {e}");
                        }
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

        events.clear();

        /*if let Err(e) = process_packets(&world, &storage).await {
            info!("oop error process_packets: {e}");
        }*/

        /*if let Err(e) = process_tasks(&world, &storage).await {
            info!("oop error process_tasks: {e}");
        }*/
        //std::thread::sleep(std::time::Duration::from_millis(1));
    }

    Ok(())
}
