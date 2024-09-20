use crate::{
    containers::*,
    gametypes::*,
    items::{ItemData, ShopData},
    maps::{GridAttribute, GridTile, MapAttribute, *},
    npcs::NpcData,
    IDIncomming,
};
use core::hint::spin_loop;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use std::{
    //collections::HashMap,
    fs,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub struct Storage {
    #[allow(clippy::type_complexity)]
    pub npc_count: Arc<AtomicU64>,
    pub player_count: Arc<AtomicU64>,
    pub map_senders: IndexMap<MapPosition, mpsc::Sender<MapIncomming>>,
    pub map_broadcast_tx: broadcast::Sender<MapBroadCasts>,
    pub id_sender: mpsc::Sender<IDIncomming>,
    pub pgconn: PgPool,
    pub bases: Arc<Bases>,
    pub config: Arc<Config>,
}

async fn establish_connection(config: &Config) -> Result<PgPool> {
    let mut connect_opts = PgConnectOptions::new();
    connect_opts = connect_opts.log_statements(log::LevelFilter::Debug);
    connect_opts = connect_opts.database(&config.sql_database);
    connect_opts = connect_opts.username(&config.sql_username);
    connect_opts = connect_opts.password(&config.sql_password);
    connect_opts = connect_opts.host(&config.sql_host);
    connect_opts = connect_opts.port(config.sql_port);

    Ok(PgPoolOptions::new()
        .max_connections(5)
        .connect_with(connect_opts)
        .await?)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerLevelFilter {
    /// A level lower than all log levels.
    Off,
    /// Corresponds to the `Error` log level.
    Error,
    /// Corresponds to the `Warn` log level.
    Warn,
    /// Corresponds to the `Info` log level.
    Info,
    /// Corresponds to the `Debug` log level.
    Debug,
    /// Corresponds to the `Trace` log level.
    Trace,
}

impl ServerLevelFilter {
    pub fn parse_enum(&self) -> LevelFilter {
        match self {
            ServerLevelFilter::Off => LevelFilter::Off,
            ServerLevelFilter::Error => LevelFilter::Error,
            ServerLevelFilter::Warn => LevelFilter::Warn,
            ServerLevelFilter::Info => LevelFilter::Info,
            ServerLevelFilter::Debug => LevelFilter::Debug,
            ServerLevelFilter::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub login_server_ip: String,
    pub login_server_port: u16,
    pub login_server_secure_port: u16,
    pub listen_ip: String,
    pub listen_port: u16,
    pub sql_database: String,
    pub sql_username: String,
    pub sql_password: String,
    pub sql_host: String,
    pub sql_port: u16,
    pub enable_backtrace: bool,
    pub level_filter: ServerLevelFilter,
    pub max_npcs: u64,
    pub max_players: u64,
    pub map_buffer_size: usize,
    pub map_broadcast_buffer_size: usize,
    pub ipc_name: String,
    pub server_name: String,
}

pub fn read_config(path: &str) -> Config {
    let data = fs::read_to_string(path).unwrap();
    toml::from_str(&data).unwrap()
}

impl Storage {
    pub async fn new(config: Config, id_sender: mpsc::Sender<IDIncomming>) -> Option<Self> {
        let pgconn = establish_connection(&config).await.unwrap();
        let mut bases = Bases::new()?;
        let (map_broadcast_tx, _map_broadcast_rx) =
            broadcast::channel(config.map_broadcast_buffer_size);

        crate::maps::load_maps().into_iter().for_each(|map_data| {
            bases.maps.insert(map_data.position, Arc::new(map_data));
        });

        crate::npcs::load_npcs()
            .into_iter()
            .enumerate()
            .for_each(|(index, npc_data)| {
                bases.npcs[index] = Arc::new(npc_data);
            });

        crate::items::load_items()
            .into_iter()
            .enumerate()
            .for_each(|(index, item_data)| {
                bases.items[index] = Arc::new(item_data);
            });

        crate::items::load_shops()
            .into_iter()
            .enumerate()
            .for_each(|(index, shopdata)| {
                bases.shops[index] = Arc::new(shopdata);
            });

        Some(Self {
            npc_count: Arc::new(AtomicU64::new(0)),
            player_count: Arc::new(AtomicU64::new(0)),
            map_senders: IndexMap::default(),
            pgconn,
            id_sender,
            map_broadcast_tx,
            bases: Arc::new(bases),
            config: Arc::new(config),
        })
    }

    pub fn get_map_sockets(
        &self,
    ) -> (
        IndexMap<MapPosition, mpsc::Sender<MapIncomming>>,
        IndexMap<MapPosition, mpsc::Receiver<MapIncomming>>,
    ) {
        let mut senders = IndexMap::default();
        let mut receivers = IndexMap::default();

        for (position, _) in &self.bases.maps {
            let (tx, rx) = mpsc::channel(self.config.map_buffer_size);

            senders.insert(*position, tx);
            receivers.insert(*position, rx);
        }

        (senders, receivers)
    }

    pub async fn generate_world_actors(&mut self) -> Result<()> {
        let (senders, mut receivers) = self.get_map_sockets();

        self.map_senders = senders;

        for (position, map_data) in &self.bases.maps {
            if let Some(receiver) = receivers.swap_remove(position) {
                let mut map = MapActor::new(
                    *position,
                    self.clone(),
                    self.map_broadcast_tx.subscribe(),
                    receiver,
                );

                let mut move_grid = [GridTile::default(); MAP_MAX_X * MAP_MAX_Y];

                for (id, grid) in move_grid.iter_mut().enumerate() {
                    match &map_data.attribute[id] {
                        MapAttribute::Blocked | MapAttribute::Storage | MapAttribute::Shop(_) => {
                            grid.attr = GridAttribute::Blocked;
                        }
                        MapAttribute::NpcBlocked => {
                            grid.attr = GridAttribute::NpcBlock;
                        }
                        MapAttribute::ItemSpawn(itemdata) => {
                            map.add_spawnable_item(
                                Position::new(id as i32 % 32, id as i32 / 32, map_data.position),
                                itemdata.index,
                                itemdata.amount,
                                itemdata.timer,
                            );
                        }
                        _ => {}
                    }
                    grid.dir_block = map_data.dir_block[id];
                }

                map.move_grids.insert(map.position, move_grid);
                tokio::spawn(map.runner());
            }
        }

        let game_time_actor = GameTimeActor::new(self.map_broadcast_tx.clone());
        tokio::spawn(game_time_actor.runner());

        Ok(())
    }

    pub fn npc_count_add(&mut self, amount: u64) {
        let mut len = self.npc_count.load(Ordering::SeqCst);
        let mut new_len = len.saturating_add(amount);

        while self
            .npc_count
            .compare_exchange(len, new_len, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            spin_loop();
            len = self.npc_count.load(Ordering::SeqCst);
            new_len = len.saturating_add(amount);
        }
    }

    pub fn npc_count_remove(&mut self, amount: u64) {
        let mut len = self.npc_count.load(Ordering::SeqCst);
        let mut new_len = len.saturating_sub(amount);

        while self
            .npc_count
            .compare_exchange(len, new_len, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            spin_loop();
            len = self.npc_count.load(Ordering::SeqCst);
            new_len = len.saturating_sub(amount);
        }
    }

    pub fn player_count_add(&mut self, amount: u64) {
        let mut len = self.player_count.load(Ordering::SeqCst);
        let mut new_len = len.saturating_add(amount);

        while self
            .player_count
            .compare_exchange(len, new_len, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            spin_loop();
            len = self.player_count.load(Ordering::SeqCst);
            new_len = len.saturating_add(amount);
        }
    }

    pub fn player_count_remove(&mut self, amount: u64) {
        let mut len = self.player_count.load(Ordering::SeqCst);
        let mut new_len = len.saturating_sub(amount);

        while self
            .player_count
            .compare_exchange(len, new_len, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            spin_loop();
            len = self.player_count.load(Ordering::SeqCst);
            new_len = len.saturating_sub(amount);
        }
    }

    pub fn get_npc_count(&mut self) -> u64 {
        self.npc_count.load(Ordering::SeqCst)
    }

    pub fn get_player_count(&mut self) -> u64 {
        self.player_count.load(Ordering::SeqCst)
    }

    pub fn get_npc(&self, id: u64) -> Option<Arc<NpcData>> {
        self.bases.npcs.get(id as usize).cloned()
    }

    pub fn get_shop(&self, id: u64) -> Option<Arc<ShopData>> {
        self.bases.shops.get(id as usize).cloned()
    }

    pub fn get_item(&self, id: u64) -> Option<Arc<ItemData>> {
        self.bases.items.get(id as usize).cloned()
    }

    pub fn get_map(&self, map_position: &MapPosition) -> Option<Arc<Map>> {
        self.bases.maps.get(map_position).cloned()
    }
}
