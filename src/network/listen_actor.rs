use crate::{
    containers::Storage,
    gametypes::Result,
    logins::LoginIncomming,
    network::{Socket, SocketActor},
};
use log::trace;
use std::{backtrace::Backtrace, io, net::SocketAddr, str::FromStr, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc};

pub struct ServerListenActor {
    pub listener: TcpListener,
    pub login_tx: mpsc::Sender<LoginIncomming>,
}

impl ServerListenActor {
    #[inline]
    pub async fn new(
        storage: &Storage,
        login_tx: mpsc::Sender<LoginIncomming>,
    ) -> Result<ServerListenActor> {
        let addr = SocketAddr::new(
            std::net::IpAddr::from_str(&storage.config.listen_ip).unwrap(),
            storage.config.listen_port,
        );
        let listener = TcpListener::bind(addr).await?;

        Ok(ServerListenActor { listener, login_tx })
    }

    pub async fn runner(self) -> Result<()> {
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

            if let Err(e) = self.login_tx.send(LoginIncomming::GetSocket(socket)).await {
                return Err(crate::AscendingError::TokioMPSCLoginSendError {
                    error: Box::new(e),
                    backtrace: Box::new(Backtrace::capture()),
                });
            }
            tokio::spawn(socket_actor.process());
        }
    }
}
