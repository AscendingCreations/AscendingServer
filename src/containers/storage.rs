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
//use futures::executor::block_on;
use mio::Poll;
use rustls::{
    crypto::{ring as provider, CryptoProvider},
    pki_types::{CertificateDer, PrivateKeyDer},
    server::WebPkiClientVerifier,
    ServerConfig,
};
use serde::Deserialize;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions, PgPool,
};
use std::{cell::RefCell, collections::VecDeque, fs, io::BufReader, sync::Arc};
use tokio::runtime::Runtime;
use tokio::task;

pub struct Storage {
    //pub players: RefCell<slab::Slab<RefCell<Player>>>,
    //pub npcs: RefCell<slab::Slab<RefCell<Npc>>>,
    pub player_ids: RefCell<IndexSet<Entity>>,
    pub recv_ids: RefCell<IndexSet<Entity>>,
    pub npc_ids: RefCell<IndexSet<Entity>>,
    pub player_names: RefCell<HashMap<String, Entity>>, //for player names to ID's
    pub maps: IndexMap<MapPosition, RefCell<MapData>>,
    //This is for buffering the specific packets needing to send.
    #[allow(clippy::type_complexity)]
    pub map_cache: RefCell<IndexMap<DataTaskToken, VecDeque<(u32, ByteBuffer, bool)>>>,
    //This keeps track of what Things need sending. So we can leave it loaded and only loop whats needed.
    pub map_cache_ids: RefCell<IndexSet<DataTaskToken>>,
    pub poll: RefCell<mio::Poll>,
    pub server: RefCell<Server>,
    pub gettick: RefCell<MyInstant>,
    pub pgconn: PgPool,
    pub time: RefCell<GameTime>,
    pub map_switch_tasks: RefCell<slab::Slab<MapSwitchTasks>>, //Data Tasks For dealing with Player Warp and MapSwitch
    pub bases: Bases,
    pub rt: RefCell<Runtime>,
    pub local: RefCell<task::LocalSet>,
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

#[derive(Deserialize)]
struct Config {
    listen: String,
    certs: String,
    server_key: String,
    client_key: String,
    maxconnections: usize,
    database: String,
    username: String,
    password: String,
    host: String,
    port: u16,
}

fn read_config(path: &str) -> Config {
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

fn build_tls_config(certs_path: &str, key_path: &str) -> Result<Arc<rustls::ServerConfig>> {
    let client_auth = WebPkiClientVerifier::no_client_auth();
    let certs = load_certs(certs_path);
    let private_key = load_private_key(key_path);

    let config = ServerConfig::builder_with_provider(
        CryptoProvider {
            cipher_suites: provider::ALL_CIPHER_SUITES.to_vec(),
            ..provider::default_provider()
        }
        .into(),
    )
    .with_protocol_versions(rustls::ALL_VERSIONS)
    .expect("inconsistent cipher-suites/versions specified")
    .with_client_cert_verifier(client_auth)
    .with_single_cert(certs, private_key)
    .expect("bad certificates/private key");

    Ok(Arc::new(config))
}

impl Storage {
    pub fn new() -> Option<Self> {
        let mut poll = Poll::new().ok()?;
        let config = read_config("settings.toml");
        let tls_config = build_tls_config(&config.certs, &config.server_key).unwrap();
        let server =
            Server::new(&mut poll, &config.listen, config.maxconnections, tls_config).ok()?;

        let mut rt: Runtime = Runtime::new().unwrap();
        let local = task::LocalSet::new();
        let pgconn = establish_connection(&config, &mut rt, &local).unwrap();
        crate::sql::initiate(&pgconn, &mut rt, &local).unwrap();

        let storage = Self {
            //we will just comment it out for now as we go along and remove later.
            //players: RefCell::new(slab::Slab::new()),
            //npcs: RefCell::new(slab::Slab::new()),
            player_ids: RefCell::new(IndexSet::default()),
            recv_ids: RefCell::new(IndexSet::default()),
            npc_ids: RefCell::new(IndexSet::default()),
            player_names: RefCell::new(HashMap::default()), //for player names to ID's
            maps: IndexMap::default(),
            map_cache: RefCell::new(IndexMap::default()),
            map_cache_ids: RefCell::new(IndexSet::default()),
            poll: RefCell::new(poll),
            server: RefCell::new(server),
            gettick: RefCell::new(MyInstant::now()),
            pgconn,
            time: RefCell::new(GameTime::default()),
            map_switch_tasks: RefCell::new(slab::Slab::new()),
            bases: Bases::new()?,
            rt: RefCell::new(rt),
            local: RefCell::new(local),
        };

        Some(storage)
    }

    pub fn add_empty_player(
        &self,
        world: &mut hecs::World,
        id: usize,
        addr: String,
    ) -> Result<Entity> {
        let socket = Socket::new(id, addr)?;

        let identity = world.spawn((
            WorldEntityType::Player,
            socket,
            OnlineType::Accepted,
            Position::default(),
        ));
        let _ = world.insert_one(identity, EntityType::Player(Entity(identity), 0));

        Ok(Entity(identity))
    }

    pub fn add_player_data(&self, world: &mut hecs::World, entity: &Entity) {
        let _ = world.insert(
            entity.0,
            (
                Account::default(),
                PlayerItemTimer::default(),
                PlayerMapTimer::default(),
                Inventory::default(),
                Equipment::default(),
                Sprite::default(),
                Money::default(),
                crate::players::MapSwitchTasks::default(),
                Player::default(),
                Spawn::default(),
                Target::default(),
                KillCount::default(),
                Vitals::default(),
                Dir::default(),
                AttackTimer::default(),
            ),
        );
        let _ = world.insert(
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
            ),
        );
    }

    pub fn remove_player(&self, world: &mut hecs::World, id: Entity) -> Option<(Socket, Position)> {
        // only removes the Components in the Fisbone ::<>
        let ret = world.remove::<(Socket, Position)>(id.0).ok();
        let account = world.remove::<(Account,)>(id.0).ok();
        //Removes Everything related to the Entity.
        let _ = world.despawn(id.0);

        if let Some((account,)) = account {
            self.player_names.borrow_mut().remove(&account.username);
        }

        self.player_ids.borrow_mut().swap_remove(&id);
        ret
    }

    pub fn add_npc(&self, world: &mut hecs::World, npc_id: u64) -> Result<Entity> {
        let npcdata = NpcData::load_npc(npc_id).expect("Cannot find NPC");

        let identity = world.spawn((
            WorldEntityType::Npc,
            Position::default(),
            NpcIndex::default(),
            NpcTimer::default(),
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
            (Spawn::default(), NpcMode::Normal, Hidden::default()),
        )?;

        if !npcdata.behaviour.is_friendly() {
            world
                .insert(
                    identity,
                    (
                        Level::default(),
                        Vitals::default(),
                        NpcHitBy::default(),
                        Target::default(),
                        AttackTimer::default(),
                        DeathTimer::default(),
                        Combat::default(),
                        Physical::default(),
                        Stunned::default(),
                        Attacking::default(),
                        InCombat::default(),
                    ),
                )
                .expect("Failed to add additional NPC Data");
        }
        world.insert_one(identity, EntityType::Npc(Entity(identity)))?;

        Ok(Entity(identity))
    }

    pub fn remove_npc(&self, world: &mut hecs::World, id: Entity) -> Option<Position> {
        let ret: Position = world.cloned_get_or_panic::<Position>(&id);
        //Removes Everything related to the Entity.
        let _ = world.despawn(id.0);
        self.npc_ids.borrow_mut().swap_remove(&id);

        //Removes the NPC from the block map.
        //TODO expand this to support larger npc's liek bosses basedon their Block size.
        self.maps
            .get(&ret.map)?
            .borrow_mut()
            .remove_entity_from_grid(ret);

        Some(ret)
    }
}
