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
use diesel::prelude::*;
use mio::Poll;
use std::cell::RefCell;

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
    pub map_cache: RefCell<IndexMap<DataTaskToken, Vec<(u32, ByteBuffer, bool)>>>,
    //This keeps track of what Things need sending. So we can leave it loaded and only loop whats needed.
    pub map_cache_ids: RefCell<IndexSet<DataTaskToken>>,
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
            pgconn: RefCell::new(establish_connection()),
            time: RefCell::new(GameTime::default()),
            map_switch_tasks: RefCell::new(slab::Slab::new()),
            bases: Bases::new()?,
        })
    }

    /*pub fn add_npc(&self, world: &mut hecs::World, npc: Npc) -> usize {
        let mut npcs = self.npcs.borrow_mut();
        let id = npcs.insert(RefCell::new(npc));
        let npc = npcs.get_mut(id).unwrap();

        //npc.borrow_mut().e.etype = EntityType::Npc(id as u64);
        // self.npc_ids.borrow_mut().insert(id);
        id
    }

    pub fn remove_npc(&self, world: &mut hecs::World, id: usize) -> Option<Npc> {
        if !self.npcs.borrow().contains(id) {
            return None;
        }

        let removed = self.npcs.borrow_mut().remove(id).into_inner();
        //self.npc_ids.borrow_mut().remove(&id);

        /*self.maps
        .get(&removed.e.pos.map)?
        .borrow_mut()
        .remove_entity_from_grid(removed.e.pos);*/
        Some(removed)
    }*/

    //lets just add the starter parts this will help use change player data around later if they where booted due to
    //bad connection but reconnected right away. this should help prevent login issues due to account is still logged in.
    /*pub fn add_player(&self, world: &mut hecs::World, id: usize, addr: String) -> Result<Entity> {
        let socket = Socket::new(id, addr)?;

        let identity = world.spawn((WorldEntityType::Player, socket, OnlineType::Accepted));
        world.insert(identity, (EntityType::Player(Entity(identity), 0)))


        player.e.etype = EntityType::Player(Entity(identity), 0);
        self.player_names
            .borrow_mut()
            .insert(player.name.clone(), id);
        self.player_ids.borrow_mut().insert(id);
        id
    }*/

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
        Some(ret)
    }
}
