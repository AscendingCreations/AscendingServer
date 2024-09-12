use crate::{
    containers::{Config, IndexMap, Storage},
    gametypes::Result,
    maps::MapIncomming,
    network::{Socket, SocketActor},
    MapPosition,
};
use log::trace;
use sqlx::PgPool;
use std::{
    io,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc},
};

pub enum LoginIncomming {
    GetSocket(Socket),
}

pub struct LoginActor {
    pub npc_count: Arc<AtomicU64>,
    pub player_count: Arc<AtomicU64>,
    pub pgconn: PgPool,
    pub login_rx: mpsc::Receiver<LoginIncomming>,
    pub senders: IndexMap<MapPosition, mpsc::Sender<MapIncomming>>,
    pub clients: Vec<Socket>,
    pub config: Arc<Config>,
}

impl LoginActor {
    pub fn new(storage: &Storage, login_rx: mpsc::Receiver<LoginIncomming>) -> Self {
        Self {
            pgconn: storage.pgconn.clone(),
            login_rx,
            senders: storage.map_senders.clone(),
            clients: Vec::with_capacity(100),
            npc_count: storage.npc_count.clone(),
            player_count: storage.player_count.clone(),
            config: storage.config.clone(),
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        Ok(())
    }
}
