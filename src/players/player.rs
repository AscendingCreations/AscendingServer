use crate::{
    gametypes::*,
    items::*,
    maps::{MapActor, MapActorStore},
    network::*,
    sql::*,
    tasks::*,
    time_ext::*,
    GlobalKey,
};
use educe::Educe;

#[derive(Clone, Debug, Educe)]
#[educe(Default)]
pub struct Trade {
    #[educe(Default = (0..MAX_TRADE_SLOT).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
    pub status: TradeStatus,
    pub vals: u64,
}

#[derive(Copy, Clone, Debug, Educe)]
#[educe(Default)]
pub struct TradeRequest {
    #[educe(Default = Target::None)]
    pub entity: crate::Target,
    #[educe(Default = MyInstant::now())]
    pub requesttimer: MyInstant,
}

#[derive(Debug, Educe)]
#[educe(Default)]
pub struct Player {
    pub uid: i64,
    pub key: GlobalKey,
    pub username: String,
    pub email: String,
    pub vals: u64,
    pub levelexp: u64,
    pub useditemid: u32,
    pub resetcount: i16,
    pub pvpon: bool,
    pub pk: bool,
    pub movesavecount: u16,
    pub sprite: u16,
    #[educe(Default = (0..MAX_EQPT).map(|_| Item::default()).collect())]
    pub equipment: Vec<Item>,
    #[educe(Default = (0..MAX_STORAGE).map(|_| Item::default()).collect())]
    pub storage: Vec<Item>,
    #[educe(Default = (0..MAX_INV).map(|_| Item::default()).collect())]
    pub inventory: Vec<Item>,
    #[educe(Default = MyInstant::now())]
    pub mapitemtimer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub itemtimer: MyInstant,
    #[educe(Default = Position::new(10, 10, MapPosition::new(0,0,0)))]
    pub spawn_pos: Position,
    #[educe(Default  = MyInstant::now())]
    pub just_spawned: MyInstant,
    pub target: Targeting,
    pub kill_count: u32,
    #[educe(Default = MyInstant::now())]
    pub kill_count_timer: MyInstant,
    #[educe(Default = [25, 2, 100])]
    pub vital: [i32; VITALS_MAX],
    #[educe(Default = [25, 2, 100])]
    pub vitalmax: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub vitalbuffs: [i32; VITALS_MAX],
    #[educe(Default = [0; VITALS_MAX])]
    pub regens: [u32; VITALS_MAX],
    pub dir: Dir,
    #[educe(Default = MyInstant::now())]
    pub despawn_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub attack_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub death_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub move_timer: MyInstant,
    #[educe(Default = MyInstant::now())]
    pub combat_timer: MyInstant,
    pub damage: u32,
    pub defense: u32,
    pub data: [i64; 10],
    pub hidden: bool,
    pub stunned: bool,
    pub attacking: bool,
    pub in_combat: bool,
    #[educe(Default = 1)]
    pub level: i32,
    pub position: Position,
    pub access: UserAccess,
    pub death: Death,
    pub is_using: IsUsingType,
    pub switch_tasks: Option<PlayerSwitchTasks>,
    //needs to be optional or we cant build a player until we have it.
    pub socket: Option<Socket>,
    pub trade: Option<Trade>,
    pub trade_request: Option<TradeRequest>,
}

impl Clone for Player {
    fn clone(&self) -> Self {
        Self {
            uid: self.uid,
            key: self.key,
            username: self.username.clone(),
            email: self.email.clone(),
            vals: self.vals,
            levelexp: self.levelexp,
            useditemid: self.useditemid,
            resetcount: self.resetcount,
            pvpon: self.pvpon,
            pk: self.pk,
            movesavecount: self.movesavecount,
            sprite: self.sprite,
            equipment: self.equipment.clone(),
            storage: self.storage.clone(),
            inventory: self.inventory.clone(),
            mapitemtimer: self.mapitemtimer,
            itemtimer: self.itemtimer,
            spawn_pos: self.spawn_pos,
            just_spawned: self.just_spawned,
            target: self.target,
            kill_count: self.kill_count,
            kill_count_timer: self.kill_count_timer,
            vital: self.vital,
            vitalmax: self.vitalmax,
            vitalbuffs: self.vitalbuffs,
            regens: self.regens,
            dir: self.dir,
            despawn_timer: self.despawn_timer,
            attack_timer: self.attack_timer,
            death_timer: self.death_timer,
            move_timer: self.move_timer,
            combat_timer: self.combat_timer,
            damage: self.damage,
            defense: self.defense,
            data: self.data,
            hidden: self.hidden,
            stunned: self.stunned,
            attacking: self.attacking,
            in_combat: self.in_combat,
            level: self.level,
            position: self.position,
            access: self.access,
            death: self.death,
            is_using: self.is_using,
            switch_tasks: None,
            socket: None,
            trade: None,
            trade_request: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerInfo {
    pub key: GlobalKey,
    pub position: Position,
}

impl PlayerInfo {
    pub fn new(key: GlobalKey, position: Position) -> Self {
        Self { key, position }
    }

    pub fn is_dead(&self, map: &MapActor, store: &MapActorStore) -> bool {
        if self.position.map == map.position {
            if let Some(player) = store.players.get(&self.key) {
                player.death.is_dead()
            } else {
                // he died and got removed or logged out.
                false
            }
        } else {
            // its not on this map so we assume its alive.
            true
        }
    }
}

#[derive(Clone, Debug, Educe)]
#[educe(Default)]
pub struct PlayerMapInfo {
    pub key: GlobalKey,
    pub dir: Dir,
    pub position: Position,
    pub access: UserAccess,
    pub death: Death,
    pub is_using: IsUsingType,
    pub hidden: bool,
    pub damage: u32,
    pub defense: u32,
    #[educe(Default = (0..MAX_EQPT).map(|_| Item::default()).collect())]
    pub equipment: Vec<Item>,
    pub level: i32,
}

impl PlayerMapInfo {
    pub fn new_from(player: Player) -> Self {
        Self {
            key: player.key,
            dir: player.dir,
            position: player.position,
            access: player.access,
            death: player.death,
            is_using: player.is_using,
            hidden: player.hidden,
            damage: player.damage,
            defense: player.defense,
            equipment: player.equipment.clone(),
            level: player.level,
        }
    }
}

/*
#[inline(always)]
pub async fn player_switch_maps(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    new_pos: Position,
) -> Result<(Position, bool)> {
    let old_position = world.get_or_err::<Position>(entity).await?;

    if let Some(mapref) = storage.maps.get(&old_position.map) {
        let mut map = mapref.write().await;
        map.remove_player(storage, *entity).await;
        map.remove_entity_from_grid(old_position);
    } else {
        return Ok((old_position, false));
    }

    if let Some(mapref) = storage.maps.get(&new_pos.map) {
        let mut map = mapref.write().await;
        map.add_player(storage, *entity).await;
        map.add_entity_to_grid(new_pos);
    } else {
        if let Some(mapref) = storage.maps.get(&old_position.map) {
            let mut map = mapref.write().await;
            map.add_player(storage, *entity).await;
            map.add_entity_to_grid(old_position);
        }

        return Ok((old_position, false));
    }

    let lock = world.write().await;
    *lock.get::<&mut Position>(entity.0)? = new_pos;

    Ok((old_position, true))
}

#[inline(always)]
pub async fn player_swap_pos(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    pos: Position,
) -> Result<Position> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        let old_position = *player_position;

        if old_position != pos {
            *player_position = pos;

            let mut map = match storage.maps.get(&old_position.map) {
                Some(map) => map,
                None => return Ok(old_position),
            }
            .write()
            .await;
            map.remove_entity_from_grid(old_position);
            map.add_entity_to_grid(pos);
        }

        old_position
    } else {
        Position::default()
    })
}

pub async fn player_add_up_vital(
    world: &GameWorld,
    entity: &crate::GlobalKey,
    vital: usize,
) -> Result<i32> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Vitals>(entity.0)?;

    Ok(if let Some(player_vital) = query.get() {
        let hp = player_vital.vitalmax[vital].saturating_add(player_vital.vitalbuffs[vital]);

        if hp.is_negative() || hp == 0 {
            1
        } else {
            hp
        }
    } else {
        1
    })
}

#[inline(always)]
pub async fn player_set_dir(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    dir: u8,
) -> Result<()> {
    let lock = world.read().await;
    let mut query = lock.query_one::<(&mut Dir, &Position)>(entity.0)?;

    if let Some((player_dir, player_position)) = query.get() {
        if player_dir.0 != dir {
            player_dir.0 = dir;

            DataTaskToken::Dir(player_position.map)
                .add_task(storage, dir_packet(*entity, dir)?)
                .await?;
        }
    }

    Ok(())
}

pub async fn player_getx(world: &GameWorld, entity: &crate::GlobalKey) -> Result<i32> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.x
    } else {
        0
    })
}

pub async fn player_gety(world: &GameWorld, entity: &crate::GlobalKey) -> Result<i32> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.y
    } else {
        0
    })
}

pub async fn player_getmap(world: &GameWorld, entity: &crate::GlobalKey) -> Result<MapPosition> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.map
    } else {
        MapPosition::new(0, 0, 0)
    })
}

pub async fn player_gethp(world: &GameWorld, entity: &crate::GlobalKey) -> Result<i32> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Vitals>(entity.0)?;

    Ok(if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize]
    } else {
        0
    })
}

pub async fn player_setx(world: &GameWorld, entity: &crate::GlobalKey, x: i32) -> Result<()> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.x = x;
    }

    Ok(())
}

pub async fn player_sety(world: &GameWorld, entity: &crate::GlobalKey, y: i32) -> Result<()> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.y = y;
    }

    Ok(())
}

pub async fn player_setmap(
    world: &GameWorld,
    entity: &crate::GlobalKey,
    map: MapPosition,
) -> Result<()> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.map = map;
    }

    Ok(())
}

pub async fn player_set_vital(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    vital: VitalTypes,
    amount: i32,
) -> Result<()> {
    {
        let lock = world.write().await;
        let mut query = lock.query_one::<&mut Vitals>(entity.0)?;

        if let Some(player_vital) = query.get() {
            player_vital.vital[vital as usize] = amount.min(player_vital.vitalmax[vital as usize]);
        }
    }

    DataTaskToken::Vitals(world.get_or_default::<Position>(entity).await.map)
        .add_task(storage, {
            let vitals = world.get_or_err::<Vitals>(entity).await?;

            vitals_packet(*entity, vitals.vital, vitals.vitalmax)?
        })
        .await?;

    Ok(())
}

#[inline]
pub async fn player_give_vals(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    amount: u64,
) -> Result<u64> {
    let player_money = world.get_or_err::<Money>(entity).await?;
    let rem = u64::MAX.saturating_sub(player_money.vals);

    if rem > 0 {
        let mut cur = amount;
        if rem >= cur {
            {
                let money = world
                    .get_or_err::<Money>(entity)
                    .await?
                    .vals
                    .saturating_add(cur);
                let lock = world.write().await;
                lock.get::<&mut Money>(entity.0)?.vals = money;
            }
            cur = 0;
        } else {
            {
                let lock = world.write().await;
                lock.get::<&mut Money>(entity.0)?.vals = u64::MAX;
            }
            cur = cur.saturating_sub(rem);
        }

        send_money(world, storage, entity).await?;
        storage
            .sql_request
            .send(SqlRequests::Currency(*entity))
            .await?;
        send_fltalert(
            storage,
            {
                let lock = world.read().await;
                let id = lock.get::<&Socket>(entity.0)?.id;
                id
            },
            format!("You Have Received {} Vals.", amount - cur),
            FtlType::Money,
        )
        .await?;
        return Ok(cur);
    }

    Ok(amount)
}

#[inline]
pub async fn player_take_vals(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::GlobalKey,
    amount: u64,
) -> Result<()> {
    let mut cur = amount;

    let player_money = world.get_or_err::<Money>(entity).await?;
    if player_money.vals >= cur {
        {
            let money = world
                .get_or_err::<Money>(entity)
                .await?
                .vals
                .saturating_sub(cur);
            let lock = world.write().await;
            lock.get::<&mut Money>(entity.0)?.vals = money;
        }
    } else {
        cur = player_money.vals;
        {
            let lock = world.write().await;
            lock.get::<&mut Money>(entity.0)?.vals = 0;
        }
    }

    send_money(world, storage, entity).await?;
    storage
        .sql_request
        .send(SqlRequests::Currency(*entity))
        .await?;
    send_fltalert(
        storage,
        {
            let lock = world.read().await;
            let id = lock.get::<&Socket>(entity.0)?.id;
            id
        },
        format!("You Lost {} Vals.", cur),
        FtlType::Money,
    )
    .await
}

pub async fn send_swap_error(
    _world: &GameWorld,
    storage: &GameStore,
    old_socket_id: usize,
    socket_id: usize,
) -> Result<()> {
    send_infomsg(
        storage,
        old_socket_id,
        "Server Error in player swap".into(),
        1,
    )
    .await?;

    send_infomsg(storage, socket_id, "Server Error in player swap".into(), 1).await
}

pub async fn send_login_info(
    world: &GameWorld,
    storage: &GameStore,
    entity: &GlobalKey,
    code: String,
    handshake: String,
    socket_id: usize,
    username: String,
) -> Result<()> {
    {
        let mut lock = world.write().await;

        lock.insert(
            entity.0,
            (
                ReloginCode {
                    code: code.to_owned(),
                },
                LoginHandShake {
                    handshake: handshake.to_owned(),
                },
            ),
        )?;
    }

    storage
        .player_usernames
        .write()
        .await
        .insert(username, *entity);
    send_myindex(storage, socket_id, entity).await?;
    send_codes(world, storage, entity, code, handshake).await
}
*/
