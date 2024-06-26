use crate::{
    containers::{Bases, HashMap, IndexMap, IndexSet},
    gametypes::*,
    maps::*,
    npcs::*,
    players::*,
    socket::*,
    tasks::{DataTaskToken, MapSwitchTasks},
    time_ext::MyInstant,
};
use chrono::Duration;
use hecs::World;
use log::LevelFilter;
use mio::Poll;
use rustls::{
    crypto::{ring as provider, CryptoProvider},
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use std::{cell::RefCell, collections::VecDeque, fs, io::BufReader, sync::Arc};
use tokio::runtime::Runtime;
use tokio::task;

pub struct Storage {
    pub player_ids: RefCell<IndexSet<Entity>>,
    pub recv_ids: RefCell<IndexSet<Entity>>,
    pub npc_ids: RefCell<IndexSet<Entity>>,
    pub player_names: RefCell<HashMap<String, Entity>>, //for player names to ID's
    pub maps: IndexMap<MapPosition, RefCell<MapData>>,
    pub map_items: RefCell<IndexMap<Position, Entity>>,
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
    pub map_switch_tasks: RefCell<IndexMap<Entity, Vec<MapSwitchTasks>>>, //Data Tasks For dealing with Player Warp and MapSwitch
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
        let server =
            Server::new(&mut poll, &config.listen, config.maxconnections, tls_config).ok()?;

        let mut rt: Runtime = Runtime::new().unwrap();
        let local = task::LocalSet::new();
        let pgconn = establish_connection(&config, &mut rt, &local).unwrap();
        crate::sql::initiate(&pgconn, &mut rt, &local).unwrap();

        let mut storage = Self {
            player_ids: RefCell::new(IndexSet::default()),
            recv_ids: RefCell::new(IndexSet::default()),
            npc_ids: RefCell::new(IndexSet::default()),
            player_names: RefCell::new(HashMap::default()), //for player names to ID's
            maps: IndexMap::default(),
            map_items: RefCell::new(IndexMap::default()),
            packet_cache: RefCell::new(IndexMap::default()),
            packet_cache_ids: RefCell::new(IndexSet::default()),
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

    pub fn add_empty_player(&self, world: &mut World, id: usize, addr: String) -> Result<Entity> {
        let socket = Socket::new(id, addr)?;

        let identity = world.spawn((WorldEntityType::Player, socket, OnlineType::Accepted));
        world.insert_one(identity, EntityType::Player(Entity(identity), 0))?;

        Ok(Entity(identity))
    }

    pub fn add_player_data(
        &self,
        world: &mut World,
        entity: &Entity,
        code: String,
        handshake: String,
        time: MyInstant,
    ) -> Result<()> {
        world.insert(
            entity.0,
            (
                Account::default(),
                PlayerItemTimer::default(),
                PlayerMapTimer::default(),
                Inventory::default(),
                Equipment::default(),
                Sprite::default(),
                Money::default(),
                Player::default(),
                Spawn::default(),
                Target::default(),
                KillCount::default(),
                Vitals::default(),
                Dir::default(),
                AttackTimer::default(),
                WorldEntityType::Player,
            ),
        )?;
        world.insert(
            entity.0,
            (
                DeathTimer::default(),
                MoveTimer::default(),
                Combat::default(),
                Physical::default(),
                Hidden::default(),
                Stunned::default(),
                Attacking::default(),
                Level::default(),
                InCombat::default(),
                EntityData::default(),
                UserAccess::default(),
                Position::default(),
                DeathType::default(),
                IsUsingType::default(),
                PlayerTarget::default(),
            ),
        )?;
        world.insert(
            entity.0,
            (
                PlayerStorage::default(),
                TradeItem::default(),
                ReloginCode { code },
                LoginHandShake { handshake },
                TradeMoney::default(),
                TradeStatus::default(),
                TradeRequestEntity::default(),
                ConnectionLoginTimer(time + Duration::try_milliseconds(600000).unwrap_or_default()),
            ),
        )?;
        self.player_ids.borrow_mut().insert(*entity);
        Ok(())
    }

    pub fn remove_player(
        &self,
        world: &mut World,
        id: Entity,
    ) -> Result<(Socket, Option<Position>)> {
        // only removes the Components in the Fisbone ::<>
        let (socket,) = world.remove::<(Socket,)>(id.0)?;
        let pos = world.remove::<(Position,)>(id.0).ok().map(|v| v.0);
        if let Ok((account,)) = world.remove::<(Account,)>(id.0) {
            println!("Players Disconnected : {}", &account.username);
            self.player_names.borrow_mut().remove(&account.username);
        }
        //Removes Everything related to the Entity.
        world.despawn(id.0)?;

        self.player_ids.borrow_mut().swap_remove(&id);
        Ok((socket, pos))
    }

    pub fn add_npc(&self, world: &mut World, npc_id: u64) -> Result<Option<Entity>> {
        if let Some(npcdata) = NpcData::load_npc(self, npc_id) {
            let identity = world.spawn((
                WorldEntityType::Npc,
                Position::default(),
                NpcIndex(npc_id),
                NpcTimer {
                    spawntimer: *self.gettick.borrow()
                        + Duration::try_milliseconds(npcdata.spawn_wait).unwrap_or_default(),
                    ..Default::default()
                },
                NpcAITimer::default(),
                NpcDespawns::default(),
                NpcMoving::default(),
                NpcRetreating::default(),
                NpcWalkToSpawn::default(),
                NpcMoves::default(),
                NpcSpawnedZone::default(),
                Dir::default(),
                MoveTimer::default(),
                EntityData::default(),
                Sprite::default(),
            ));
            world.insert(
                identity,
                (
                    Spawn::default(),
                    NpcMode::Normal,
                    Hidden::default(),
                    Level::default(),
                    Vitals::default(),
                    Physical::default(),
                    DeathType::default(),
                    NpcMovePos::default(),
                    Target::default(),
                    InCombat::default(),
                    AttackTimer::default(),
                    NpcPathTimer::default(),
                ),
            )?;

            if !npcdata.behaviour.is_friendly() {
                world.insert(
                    identity,
                    (
                        NpcHitBy::default(),
                        Target::default(),
                        AttackTimer::default(),
                        DeathTimer::default(),
                        Combat::default(),
                        Stunned::default(),
                        Attacking::default(),
                        InCombat::default(),
                    ),
                )?;
            }
            world.insert_one(identity, EntityType::Npc(Entity(identity)))?;

            self.npc_ids.borrow_mut().insert(Entity(identity));

            Ok(Some(Entity(identity)))
        } else {
            Ok(None)
        }
    }

    pub fn remove_npc(&self, world: &mut World, id: Entity) -> Result<Position> {
        let ret: Position = world.get_or_err::<Position>(&id)?;
        //Removes Everything related to the Entity.
        world.despawn(id.0)?;
        self.npc_ids.borrow_mut().swap_remove(&id);

        //Removes the NPC from the block map.
        //TODO expand this to support larger npc's liek bosses basedon their Block size.
        if let Some(map) = self.maps.get(&ret.map) {
            map.borrow_mut().remove_entity_from_grid(ret);
        }

        Ok(ret)
    }
}
