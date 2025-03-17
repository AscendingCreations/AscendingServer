use crate::{
    containers::{Bases, HashMap, IndexMap, IndexSet},
    gametypes::*,
    maps::*,
    npcs::*,
    socket::*,
    tasks::{DataTaskToken, MapSwitchTasks},
    time_ext::MyInstant,
};
use chrono::Duration;
use log::{LevelFilter, error, info, trace, warn};
use mio::{Poll, Token};
use rustls::{
    ServerConfig,
    crypto::{CryptoProvider, ring as provider},
    pki_types::{CertificateDer, PrivateKeyDer},
};
use serde::{Deserialize, Serialize};
use slotmap::SecondaryMap;
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    fs,
    io::BufReader,
    sync::{Arc, Mutex},
};
use tokio::runtime::Runtime;
use tokio::task;

use super::{
    CombatData, Entity, EntityKind, GlobalKey, HashSet, LoginHandShake, MovementData, NpcEntity,
    NpcMode, NpcTimer, PlayerConnectionTimer, PlayerEntity, ReloginCode, Socket, Spawn, Vitals,
    World,
};

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct ClearCodeData {
    pub code: String,
    pub timer: MyInstant,
}

pub struct Storage {
    pub player_ids: RefCell<IndexSet<GlobalKey>>,
    pub recv_ids: RefCell<IndexSet<Token>>,
    pub npc_ids: RefCell<IndexSet<GlobalKey>>,
    pub player_names: RefCell<HashMap<String, GlobalKey>>, //for player names to ID's
    pub maps: IndexMap<MapPosition, RefCell<MapData>>,
    pub map_items: RefCell<IndexMap<Position, GlobalKey>>,
    pub disconnected_player: RefCell<IndexSet<GlobalKey>>, //Players get placed here to unload data later. allow reconnecting.
    pub player_timeout: RefCell<SecondaryMap<GlobalKey, PlayerConnectionTimer>>,
    pub hand_shakes: RefCell<HashMap<String, GlobalKey>>,
    pub player_code: RefCell<IndexMap<String, GlobalKey>>,
    //Keep track of older relogin codes so we can remove them after a set period of time.
    pub clear_code: RefCell<IndexSet<ClearCodeData>>,
    //This is for buffering the specific packets needing to send.
    #[allow(clippy::type_complexity)]
    pub packet_cache: RefCell<IndexMap<DataTaskToken, VecDeque<(u32, MByteBuffer, bool)>>>,
    //This keeps track of what Things need sending. So we can leave it loaded and only loop whats needed.
    pub packet_cache_ids: RefCell<IndexSet<DataTaskToken>>,
    pub poll: RefCell<mio::Poll>,
    pub server: RefCell<Server>,
    pub gettick: RefCell<MyInstant>,
    pub pgconn: PgPool,
    pub time: RefCell<GameTime>,
    pub map_switch_tasks: RefCell<IndexMap<GlobalKey, Vec<MapSwitchTasks>>>, //Data Tasks For dealing with Player Warp and MapSwitch
    pub bases: Bases,
    pub rt: RefCell<Runtime>,
    pub local: RefCell<task::LocalSet>,
    pub config: Config,
}

fn establish_connection(
    config: &Config,
    rt: &mut Runtime,
    local: &task::LocalSet,
) -> Result<PgPool> {
    let mut connect_opts = PgConnectOptions::new();
    connect_opts = connect_opts.log_statements(log::LevelFilter::Debug);
    connect_opts = connect_opts.database(&config.database);
    connect_opts = connect_opts.username(&config.username);
    connect_opts = connect_opts.password(&config.password);
    connect_opts = connect_opts.host(&config.host);
    connect_opts = connect_opts.port(config.port);

    let pool = local.block_on(
        rt,
        PgPoolOptions::new()
            .max_connections(5)
            .connect_with(connect_opts),
    )?;

    Ok(pool)
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

#[derive(Deserialize)]
pub struct Config {
    pub listen: String,
    pub tls_listen: String,
    pub server_cert: String,
    pub server_key: String,
    pub ca_root: String,
    pub maxconnections: usize,
    pub database: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub enable_backtrace: bool,
    pub level_filter: ServerLevelFilter,
}

pub fn read_config(path: &str) -> Config {
    let data = fs::read_to_string(path).unwrap();
    toml::from_str(&data).unwrap()
}

fn load_certs(filename: &str) -> Vec<CertificateDer<'static>> {
    let certfile = fs::File::open(filename).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader)
        .map(|result| result.unwrap())
        .collect()
}

fn load_private_key(filename: &str) -> PrivateKeyDer<'static> {
    let keyfile = fs::File::open(filename).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);

    loop {
        match rustls_pemfile::read_one(&mut reader).expect("cannot parse private key .pem file") {
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => return key.into(),
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => return key.into(),
            Some(rustls_pemfile::Item::Sec1Key(key)) => return key.into(),
            None => break,
            _ => {}
        }
    }

    panic!(
        "no keys found in {:?} (encrypted keys not supported)",
        filename
    );
}

fn build_tls_config(
    server_certs_path: &str,
    server_key_path: &str,
    _ca_root_path: &str,
) -> Result<Arc<rustls::ServerConfig>> {
    let certs = load_certs(server_certs_path);
    let private_key = load_private_key(server_key_path);

    let config = ServerConfig::builder_with_provider(
        CryptoProvider {
            cipher_suites: provider::ALL_CIPHER_SUITES.to_vec(),
            ..provider::default_provider()
        }
        .into(),
    )
    .with_protocol_versions(rustls::ALL_VERSIONS)
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(certs, private_key)?;

    Ok(Arc::new(config))
}

impl Storage {
    pub fn new(config: Config) -> Option<Self> {
        let mut poll = Poll::new().ok()?;
        let tls_config =
            build_tls_config(&config.server_cert, &config.server_key, &config.ca_root).unwrap();
        let server = Server::new(
            &mut poll,
            &config.listen,
            &config.tls_listen,
            config.maxconnections,
            tls_config,
        )
        .ok()?;

        let mut rt: Runtime = Runtime::new().unwrap();
        let local = task::LocalSet::new();
        let pgconn = establish_connection(&config, &mut rt, &local).unwrap();
        crate::sql::initiate(&pgconn, &mut rt, &local).unwrap();

        let mut storage = Self {
            player_ids: RefCell::new(IndexSet::default()),
            recv_ids: RefCell::new(IndexSet::default()),
            npc_ids: RefCell::new(IndexSet::default()),
            player_names: RefCell::new(HashMap::default()), //for player names to ID's
            disconnected_player: RefCell::new(IndexSet::default()),
            player_timeout: RefCell::new(SecondaryMap::default()),
            maps: IndexMap::default(),
            map_items: RefCell::new(IndexMap::default()),
            packet_cache: RefCell::new(IndexMap::default()),
            packet_cache_ids: RefCell::new(IndexSet::default()),
            hand_shakes: RefCell::new(HashMap::default()),
            player_code: RefCell::new(IndexMap::default()),
            clear_code: RefCell::new(IndexSet::default()),
            poll: RefCell::new(poll),
            server: RefCell::new(server),
            gettick: RefCell::new(MyInstant::now()),
            pgconn,
            time: RefCell::new(GameTime::default()),
            map_switch_tasks: RefCell::new(IndexMap::default()),
            bases: Bases::new()?,
            rt: RefCell::new(rt),
            local: RefCell::new(local),
            config,
        };

        let mut map_data_entry = crate::maps::get_maps();
        while let Some(map_data) = map_data_entry.pop() {
            let position = MapPosition {
                x: map_data.position.x,
                y: map_data.position.y,
                group: map_data.position.group,
            };

            let mut map = MapData {
                position,
                ..Default::default()
            };

            for id in 0..MAP_MAX_X * MAP_MAX_Y {
                match map_data.attribute[id].clone() {
                    MapAttribute::Blocked | MapAttribute::Storage | MapAttribute::Shop(_) => {
                        map.move_grid[id].attr = GridAttribute::Blocked;
                    }
                    MapAttribute::NpcBlocked => {
                        map.move_grid[id].attr = GridAttribute::NpcBlock;
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
                map.move_grid[id].dir_block = map_data.dir_block[id];
            }

            storage.maps.insert(position, RefCell::new(map));
            storage.bases.maps.insert(position, map_data);
        }

        let npc_data_entry = crate::npcs::get_npc();
        npc_data_entry
            .iter()
            .enumerate()
            .for_each(|(index, npc_data)| {
                storage.bases.npcs[index] = npc_data.clone();
            });

        let item_data_entry = crate::items::get_item();
        item_data_entry
            .iter()
            .enumerate()
            .for_each(|(index, item_data)| {
                storage.bases.items[index] = item_data.clone();
            });

        let shop_data_entry = crate::items::get_shop();
        shop_data_entry
            .iter()
            .enumerate()
            .for_each(|(index, shopdata)| {
                storage.bases.shops[index] = shopdata.clone();
            });

        Some(storage)
    }

    pub fn add_player_data(
        &self,
        world: &mut World,
        code: String,
        handshake: String,
        socket: Socket,
    ) -> Result<GlobalKey> {
        let entity = world.kinds.insert(EntityKind::Player);
        let player_entity = create_player_entity(code, handshake, socket);

        world
            .entities
            .insert(entity, Entity::Player(Arc::new(Mutex::new(player_entity))));

        self.player_ids.borrow_mut().insert(entity);
        Ok(entity)
    }

    pub fn remove_player(
        &self,
        world: &mut World,
        id: GlobalKey,
    ) -> Result<Option<Arc<Mutex<PlayerEntity>>>> {
        let _ = world.kinds.remove(id);
        let player = world.entities.remove(id);
        self.player_ids.borrow_mut().swap_remove(&id);

        Ok(if let Some(data) = player {
            if let Entity::Player(p_data) = data {
                {
                    let p_data = p_data.try_lock()?;

                    let _ = world.account_id.remove(&p_data.account.id);

                    // pos = Some((p_data.movement.pos, p_data.movement.map_instance));

                    self.player_names
                        .borrow_mut()
                        .remove(&p_data.account.username);

                    info!("Players Disconnected : {}", &p_data.account.username);
                    trace!("Players Disconnected IP: {} ", &p_data.socket.addr);
                }

                Some(p_data)
            } else {
                error!("Was not a player entity woops?");
                None
            }
        } else {
            warn!("Player Removal failed: Cant find player");
            None
        })
    }

    pub fn add_npc(&self, world: &mut World, npc_id: u64) -> Result<Option<GlobalKey>> {
        if let Some(npcdata) = NpcData::load_npc(self, npc_id) {
            let entity = world.kinds.insert(EntityKind::Npc);

            let mut vitals = Vitals::default();

            vitals.vital[VitalTypes::Hp as usize] = npcdata.maxhp as i32;
            vitals.vitalmax[VitalTypes::Hp as usize] = npcdata.maxhp as i32;

            world.entities.insert(
                entity,
                Entity::Npc(Arc::new(Mutex::new(NpcEntity {
                    index: npc_id,
                    timer: NpcTimer {
                        spawntimer: *self.gettick.borrow()
                            + Duration::try_milliseconds(npcdata.spawn_wait).unwrap_or_default(),
                        ..Default::default()
                    },
                    combat: CombatData {
                        vitals,
                        ..Default::default()
                    },
                    mode: NpcMode::Normal,
                    ..Default::default()
                }))),
            );

            self.npc_ids.borrow_mut().insert(entity);

            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    pub fn remove_npc(&self, world: &mut World, id: GlobalKey) -> Result<Position> {
        let _ = world.kinds.remove(id);
        let npc = world.entities.remove(id);
        self.npc_ids.borrow_mut().swap_remove(&id);

        if let Some(Entity::Npc(n_data)) = npc {
            let n_data = n_data.try_lock()?;

            //Removes the NPC from the block map.
            //TODO_ON_DEV expand this to support larger npc's liek bosses basedon their Block size.
            if let Some(map) = self.maps.get(&n_data.movement.pos.map) {
                map.borrow_mut()
                    .remove_entity_from_grid(n_data.movement.pos);
            }

            return Ok(n_data.movement.pos);
        }

        Err(AscendingError::missing_entity())
    }
}

pub fn create_player_entity(code: String, handshake: String, socket: Socket) -> PlayerEntity {
    let mut hash_code = HashSet::default();
    hash_code.insert(code.to_owned());

    let start_pos = Position {
        x: 0,
        y: 0,
        map: MapPosition {
            x: 0,
            y: 0,
            group: 0,
        },
    };

    PlayerEntity {
        movement: MovementData {
            pos: start_pos,
            spawn: Spawn {
                pos: start_pos,
                ..Default::default()
            },
            ..Default::default()
        },
        login_handshake: LoginHandShake { handshake },
        relogin_code: ReloginCode { code: hash_code },
        online_type: OnlineType::Accepted,
        socket,
        ..Default::default()
    }
}
