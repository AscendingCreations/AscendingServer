use crate::{
    containers::{Bases, HashMap, IndexMap, IndexSet},
    gameloop::*,
    gametypes::*,
    maps::*,
    npcs::*,
    players::Player,
    socket::*,
    tasks::{DataTaskToken, MapSwitchTasks},
    time_ext::MyInstant,
};
use bytey::ByteBuffer;
use diesel::prelude::*;
use mio::Poll;
use std::cell::RefCell;

pub struct Storage {
    pub players: RefCell<slab::Slab<RefCell<Player>>>,
    pub npcs: RefCell<slab::Slab<RefCell<Npc>>>,
    pub player_ids: RefCell<IndexSet<usize>>,
    pub recv_ids: RefCell<IndexSet<usize>>,
    pub npc_ids: RefCell<IndexSet<usize>>,
    pub player_names: RefCell<HashMap<String, usize>>, //for player names to ID's
    pub maps: IndexMap<MapPosition, RefCell<MapData>>,
    //This is for buffering the specific packets needing to send.
    pub map_cache: RefCell<HashMap<DataTaskToken, Vec<ByteBuffer>>>,
    //This keeps track of what Things need sending. So we can leave it loaded and only loop whats needed.
    pub map_cache_ids: RefCell<IndexSet<usize>>,
    pub poll: RefCell<mio::Poll>,
    pub server: RefCell<Server>,
    pub gettick: RefCell<MyInstant>,
    pub pgconn: RefCell<PgConnection>,
    pub time: RefCell<GameTime>,
    pub map_switch_tasks: RefCell<slab::Slab<MapSwitchTasks>>, //Data Tasks For dealing with Player Warp and MapSwitch
    pub bases: Bases,
}

pub fn establish_connection() -> PgConnection {
    let database_url = "postgres://test:damit1@localhost:5432/test";
    PgConnection::establish(database_url)
        .unwrap_or_else(|_i| panic!("Error connecting to {}", database_url))
}

impl Storage {
    pub fn new() -> Option<Self> {
        let mut poll = Poll::new().ok()?;
        let server = Server::new(&mut poll, SERVERCONNECTION, MAXCONNECTIONS).ok()?;

        Some(Self {
            players: RefCell::new(slab::Slab::new()),
            npcs: RefCell::new(slab::Slab::new()),
            player_ids: RefCell::new(IndexSet::default()),
            recv_ids: RefCell::new(IndexSet::default()),
            npc_ids: RefCell::new(IndexSet::default()),
            player_names: RefCell::new(HashMap::default()), //for player names to ID's
            maps: IndexMap::default(),
            map_cache: RefCell::new(HashMap::default()),
            map_cache_ids: RefCell::new(IndexSet::default()),
            poll: RefCell::new(poll),
            server: RefCell::new(server),
            gettick: RefCell::new(MyInstant::now()),
            pgconn: RefCell::new(establish_connection()),
            time: RefCell::new(GameTime::default()),
            map_switch_tasks: RefCell::new(slab::Slab::new()),
            bases: Bases::new()?,
        })
    }

    pub fn add_npc(&self, npc: Npc) -> usize {
        let mut npcs = self.npcs.borrow_mut();
        let id = npcs.insert(RefCell::new(npc));
        let npc = npcs.get_mut(id).unwrap();

        npc.borrow_mut().e.etype = EntityType::Npc(id as u64);
        self.npc_ids.borrow_mut().insert(id);
        id
    }

    pub fn remove_npc(&self, id: usize) -> Option<Npc> {
        if !self.npcs.borrow().contains(id) {
            return None;
        }

        let removed = self.npcs.borrow_mut().remove(id).into_inner();
        self.npc_ids.borrow_mut().remove(&id);

        self.maps
            .get(&removed.e.pos.map)?
            .borrow_mut()
            .remove_entity_from_grid(removed.e.pos);
        Some(removed)
    }

    pub fn add_player(&self, player: Player) -> usize {
        let mut players = self.players.borrow_mut();
        let id = players.insert(RefCell::new(player));
        let playerref = players.get_mut(id).unwrap();
        let mut player = playerref.borrow_mut();

        player.e.etype = EntityType::Player(id as u64, player.accid);
        self.player_names
            .borrow_mut()
            .insert(player.name.clone(), id);
        self.player_ids.borrow_mut().insert(id);
        id
    }

    pub fn remove_player(&self, id: usize) -> Option<Player> {
        if !self.players.borrow().contains(id) {
            return None;
        }

        let removed = self.players.borrow_mut().remove(id);
        let _oldid = self
            .player_names
            .borrow_mut()
            .remove(&removed.borrow_mut().name);
        self.player_ids.borrow_mut().remove(&id);
        Some(removed.into_inner())
    }
}
