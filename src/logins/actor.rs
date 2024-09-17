use std::{net::SocketAddr, str::FromStr};

use crate::{containers::Storage, gametypes::Result, network::Socket};

use tokio::{
    net::{TcpSocket, TcpStream},
    sync::mpsc,
};

pub enum LoginIncomming {
    GetSocket(Socket),
}

pub struct LoginActor {
    pub login_rx: mpsc::Receiver<LoginIncomming>,
    pub login_stream: TcpStream,
    pub clients: Vec<Socket>,
    pub storage: Storage,
}

impl LoginActor {
    pub async fn new(storage: &Storage, login_rx: mpsc::Receiver<LoginIncomming>) -> Self {
        let addr = SocketAddr::new(
            std::net::IpAddr::from_str(&storage.config.login_server_ip).unwrap(),
            storage.config.login_server_port,
        );
        let socket = TcpSocket::new_v4().unwrap();
        let login_stream = socket.connect(addr).await.unwrap();

        login_stream.set_nodelay(true);

        Self {
            login_rx,
            login_stream,
            clients: Vec::with_capacity(100),
            storage: storage.clone(),
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        Ok(())
    }
}
