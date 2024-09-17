use crate::{gametypes::*, maps::MapBroadCasts, GameTime, GlobalKey, UserAccess};
use bytey::{ByteBufferRead, ByteBufferWrite};
use tokio::{
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
}

impl InfoActor {
    pub fn new(
        map_broadcast_rx: broadcast::Receiver<MapBroadCasts>,
    ) -> (mpsc::Sender<InfoIncomming>, Self) {
        let (info_tx, info_rx) = mpsc::channel(100);

        (
            info_tx,
            Self {
                map_broadcast_rx,
                usernames: HashMap::default(),
                info_rx,
                game_time: GameTime::default(),
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
                        Ok(packet) => self.update_info(packet),
                        Err(e) => {
                            log::info!("map_broadcast_rx was closed. {e}");
                            return Err(crate::AscendingError::from(e));
                        }
                    }
                }
            }
        }
    }

    pub fn update_info(&mut self, packet: MapBroadCasts) {
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
            }
            MapBroadCasts::PlayerLoggedOut {
                map_id: _,
                key: _,
                username,
                position: _,
            } => {
                self.usernames.remove(&username);
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
