use crate::{
    containers::{Config, Storage},
    gametypes::*,
    logins::LoginIncomming,
    maps::MapBroadCasts,
    network::MByteBufferExt,
    GameTime, GlobalKey, UserAccess,
};
use bytey::{ByteBufferRead, ByteBufferWrite};
use mmap_bytey::MByteBuffer;
use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpSocket, TcpStream},
    select,
    sync::{broadcast, mpsc, oneshot},
};

#[derive(Debug)]
pub enum InfoIncomming {
    None,
    GetOnlineUsers(oneshot::Sender<InfoOutGoing>),
}

#[derive(Debug, Clone)]
pub enum InfoOutGoing {
    None,
    SendOnlineUsers(Vec<PlayerInfo>),
}

#[derive(Debug, Clone, ByteBufferRead, ByteBufferWrite)]
pub struct PlayerInfo {
    pub key: GlobalKey,
    pub username: String,
    pub access: UserAccess,
}

pub struct InfoActor {
    pub map_broadcast_rx: broadcast::Receiver<MapBroadCasts>,
    pub usernames: HashMap<String, PlayerInfo>,
    pub game_time: GameTime,
    pub info_rx: mpsc::Receiver<InfoIncomming>,
    pub login_tx: mpsc::Sender<LoginIncomming>,
    pub config: Arc<Config>,
}

impl InfoActor {
    pub fn new(
        login_tx: mpsc::Sender<LoginIncomming>,
        map_broadcast_rx: broadcast::Receiver<MapBroadCasts>,
        storage: &Storage,
    ) -> (mpsc::Sender<InfoIncomming>, Self) {
        let (info_tx, info_rx) = mpsc::channel(100);

        (
            info_tx,
            Self {
                login_tx,
                map_broadcast_rx,
                usernames: HashMap::default(),
                info_rx,
                game_time: GameTime::default(),
                config: storage.config.clone(),
            },
        )
    }

    pub async fn runner(mut self) -> Result<()> {
        loop {
            select! {
                packet = self.info_rx.recv() => {
                    match packet {
                        Some(packet) => self.handle_request(packet)?,
                        None => {
                            log::info!("info_tx was closed.");
                            return Ok(());
                        }
                    }
                }
                packet = self.map_broadcast_rx.recv() => {
                    match packet {
                        Ok(packet) => self.update_info(packet).await,
                        Err(e) => {
                            log::info!("map_broadcast_rx was closed. {e}");
                            return Err(crate::AscendingError::from(e));
                        }
                    }
                }
            }
        }
    }

    pub async fn update_info(&mut self, packet: MapBroadCasts) {
        match packet {
            MapBroadCasts::PlayerLoggedIn {
                map_id: _,
                key,
                username,
                access,
                position: _,
            } => {
                self.usernames.insert(
                    username.clone(),
                    PlayerInfo {
                        key,
                        username,
                        access,
                    },
                );

                self.login_tx
                    .send(LoginIncomming::UpdateLoginServer {
                        users_len: self.usernames.len() as u64,
                    })
                    .await
                    .unwrap();
            }
            MapBroadCasts::PlayerLoggedOut {
                map_id: _,
                key: _,
                username,
                position: _,
            } => {
                self.usernames.remove(&username);

                self.login_tx
                    .send(LoginIncomming::UpdateLoginServer {
                        users_len: self.usernames.len() as u64,
                    })
                    .await
                    .unwrap();
            }
            MapBroadCasts::TimeUpdate { time } => {
                self.game_time = time;
            }
            _ => {}
        }
    }

    pub fn handle_request(&mut self, packet: InfoIncomming) -> Result<()> {
        #[allow(clippy::single_match)]
        match packet {
            InfoIncomming::GetOnlineUsers(tx) => {
                let users: Vec<PlayerInfo> = self.usernames.values().cloned().collect();

                if tx.send(InfoOutGoing::SendOnlineUsers(users)).is_err() {
                    log::trace!("Could not send InfoOutGoing as oneshot errored.");
                }
            }
            _ => {}
        }

        Ok(())
    }
}
