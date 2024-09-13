use crate::{containers::Storage, gametypes::Result, network::Socket};

use tokio::sync::mpsc;

pub enum LoginIncomming {
    GetSocket(Socket),
}

pub struct LoginActor {
    pub login_rx: mpsc::Receiver<LoginIncomming>,
    pub clients: Vec<Socket>,
    pub storage: Storage,
}

impl LoginActor {
    pub fn new(storage: &Storage, login_rx: mpsc::Receiver<LoginIncomming>) -> Self {
        Self {
            login_rx,
            clients: Vec::with_capacity(100),
            storage: storage.clone(),
        }
    }

    pub async fn runner(mut self) -> Result<()> {
        Ok(())
    }
}
