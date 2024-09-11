use crate::{
    gametypes::Result,
    network::{Socket, SocketActor},
};
use log::trace;
use std::{io, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc};

pub enum LoginIncomming {
    GetSocket(Socket),
}

pub struct LoginActor {
    pub login_rx: mpsc::Receiver<LoginIncomming>,
    pub clients: Vec<Socket>,
}

pub struct ServerListenActor {
    pub listener: TcpListener,
    pub login_tx: mpsc::Sender<LoginIncomming>,
}

impl ServerListenActor {
    #[inline]
    pub async fn new(
        addr: &str,
        login_tx: mpsc::Sender<LoginIncomming>,
    ) -> Result<ServerListenActor> {
        let listener = TcpListener::bind(addr).await?;

        Ok(ServerListenActor { listener, login_tx })
    }

    pub async fn accept(&mut self) -> Result<()> {
        /* Wait for a new connection to accept and try to grab a token from the bag. */
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok((stream, addr)) => (stream, Arc::new(addr.to_string())),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => {
                    trace!("listener.accept error: {}", e);
                    return Err(e.into());
                }
            };

            stream.set_nodelay(true)?;

            let (owned_rx, owned_tx) = stream.into_split();
            let (tx, rx) = mpsc::channel(500);
            let socket_actor = SocketActor::new(owned_rx, tx, addr.clone())?;
            let socket = Socket::new(owned_tx, rx, addr);

            self.login_tx
                .send(LoginIncomming::GetSocket(socket))
                .await?;
            tokio::spawn(socket_actor.process());
        }
    }
}
