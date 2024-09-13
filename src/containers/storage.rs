use crate::{
    containers::{Bases, IndexMap},
    gametypes::*,
    maps::*,
    time_ext::MyInstant,
};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use std::{
    fs,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub struct Storage {
    #[allow(clippy::type_complexity)]
    pub npc_count: Arc<AtomicU64>,
    pub player_count: Arc<AtomicU64>,
    pub map_senders: IndexMap<MapPosition, mpsc::Sender<MapIncomming>>,
    pub map_broadcast_tx: broadcast::Sender<MapBroadCasts>,
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
}

pub fn read_config(path: &str) -> Config {
    let data = fs::read_to_string(path).unwrap();
    toml::from_str(&data).unwrap()
}

impl Storage {
    pub async fn new(config: Config) -> Option<Self> {
        let pgconn = establish_connection(&config).await.unwrap();
        let mut bases = Bases::new()?;
        let (map_broadcast_tx, _map_broadcast_rx) =
            broadcast::channel(config.map_broadcast_buffer_size);

        crate::maps::get_maps().into_iter().for_each(|map_data| {
            bases.maps.insert(map_data.position, map_data);
        });

        crate::npcs::get_npc()
            .into_iter()
            .enumerate()
            .for_each(|(index, npc_data)| {
                bases.npcs[index] = npc_data;
            });

        crate::items::get_item()
            .into_iter()
            .enumerate()
            .for_each(|(index, item_data)| {
                bases.items[index] = item_data;
            });

        crate::items::get_shop()
            .into_iter()
            .enumerate()
            .for_each(|(index, shopdata)| {
                bases.shops[index] = shopdata;
            });

        Some(Self {
            npc_count: Arc::new(AtomicU64::new(0)),
            player_count: Arc::new(AtomicU64::new(0)),
            map_senders: IndexMap::default(),
            pgconn,
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
                let mut map = MapActor {
                    position: *position,
                    storage: self.clone(),
                    broadcast_rx: self.map_broadcast_tx.subscribe(),
                    receiver,
                    tick: MyInstant::now(),
                    zones: [0, 0, 0, 0, 0],
                    move_grid: [GridTile::default(); MAP_MAX_X * MAP_MAX_Y],
                    spawnable_item: Vec::new(),
                    move_grids: IndexMap::default(),
                };

                for id in 0..MAP_MAX_X * MAP_MAX_Y {
                    match &map_data.attribute[id] {
                        MapAttribute::Blocked | MapAttribute::Storage | MapAttribute::Shop(_) => {
                            map.move_grid[id].attr = GridAttribute::Blocked;
                        }
                        MapAttribute::NpcBlocked => {
                            map.move_grid[id].attr = GridAttribute::NpcBlock;
                        }
                        MapAttribute::ItemSpawn(_itemdata) => {
                            /* map.add_spawnable_item(
                                Position::new(id as i32 % 32, id as i32 / 32, map_data.position),
                                itemdata.index,
                                itemdata.amount,
                                itemdata.timer,
                            );*/
                        }
                        _ => {}
                    }
                    map.move_grid[id].dir_block = map_data.dir_block[id];
                }

                tokio::spawn(map.runner());
            }
        }

        let game_time_actor = GameTimeActor::new(self.map_broadcast_tx.clone());
        tokio::spawn(game_time_actor.runner());

        Ok(())
    }
}
