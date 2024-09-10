use crate::{containers::*, gametypes::*, items::*, network::*, sql::*, tasks::*, time_ext::*};
use educe::Educe;
use hecs::Bundle;
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Bundle)]
pub struct Socket {
    // IP address
    pub addr: Arc<String>,
    // Socket ID
    pub id: usize,
    // Packet Buffer
    pub buffer: Arc<RwLock<ByteBuffer>>,
}

impl Socket {
    #[inline(always)]
    pub fn new(id: usize, addr: String) -> Result<Self> {
        Ok(Self {
            id,
            addr: Arc::new(addr),
            buffer: Arc::new(RwLock::new(ByteBuffer::with_capacity(8192)?)),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub username: String,
    pub email: String,
    pub passresetcode: Option<String>,
    pub id: i64,
}

#[derive(Copy, Clone, Debug, Educe)]
#[educe(Default)]
pub struct PlayerItemTimer {
    #[educe(Default = MyInstant::now())]
    pub itemtimer: MyInstant,
}

#[derive(Copy, Clone, Debug, Educe)]
#[educe(Default)]
pub struct PlayerMapTimer {
    #[educe(Default = MyInstant::now())]
    pub mapitemtimer: MyInstant,
}

#[derive(
    PartialEq,
    Eq,
    Clone,
    Debug,
    Educe,
    Deserialize,
    Serialize,
    ByteBufferRead,
    ByteBufferWrite,
    MByteBufferRead,
    MByteBufferWrite,
)]
#[educe(Default)]
pub struct Inventory {
    #[educe(Default = (0..MAX_INV).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(
    PartialEq,
    Eq,
    Clone,
    Debug,
    Educe,
    Deserialize,
    Serialize,
    ByteBufferRead,
    ByteBufferWrite,
    MByteBufferRead,
    MByteBufferWrite,
)]
#[educe(Default)]
pub struct TradeItem {
    #[educe(Default = (0..MAX_TRADE_SLOT).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct TradeMoney {
    pub vals: u64,
}

#[derive(
    PartialEq,
    Eq,
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    ByteBufferRead,
    ByteBufferWrite,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum TradeStatus {
    #[default]
    None,
    Accepted,
    Submitted,
}

#[derive(Copy, Clone, Debug, Educe)]
#[educe(Default)]
pub struct TradeRequestEntity {
    #[educe(Default = None)]
    pub entity: Option<crate::Entity>,
    #[educe(Default = MyInstant::now())]
    pub requesttimer: MyInstant,
}

#[derive(
    PartialEq,
    Eq,
    Clone,
    Debug,
    Educe,
    Deserialize,
    Serialize,
    ByteBufferRead,
    ByteBufferWrite,
    MByteBufferRead,
    MByteBufferWrite,
)]
#[educe(Default)]
pub struct PlayerStorage {
    #[educe(Default = (0..MAX_STORAGE).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(
    PartialEq,
    Eq,
    Clone,
    Debug,
    Educe,
    Deserialize,
    Serialize,
    ByteBufferRead,
    ByteBufferWrite,
    MByteBufferRead,
    MByteBufferWrite,
)]
#[educe(Default)]
pub struct Equipment {
    #[educe(Default = (0..MAX_EQPT).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Sprite {
    pub id: u16,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Money {
    pub vals: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ReloginCode {
    pub code: String,
}

#[derive(Clone, Debug, Default)]
pub struct LoginHandShake {
    pub handshake: String,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Player {
    pub levelexp: u64,
    pub useditemid: u32,
    pub resetcount: i16,
    pub pvpon: bool,
    pub pk: bool,
    pub movesavecount: u16,
}

pub async fn is_player_online(world: &GameWorld, entity: &crate::Entity) -> Result<bool> {
    let lock = world.read().await;
    let online = *lock.get::<&WorldEntityType>(entity.0)? == WorldEntityType::Player
        && *lock.get::<&OnlineType>(entity.0)? == OnlineType::Online;

    Ok(online)
}

#[inline(always)]
pub async fn player_switch_maps(
    world: &GameWorld,
    storage: &GameStore,
    entity: &crate::Entity,
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
    entity: &crate::Entity,
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
    entity: &crate::Entity,
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
    entity: &crate::Entity,
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

pub async fn player_getx(world: &GameWorld, entity: &crate::Entity) -> Result<i32> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.x
    } else {
        0
    })
}

pub async fn player_gety(world: &GameWorld, entity: &crate::Entity) -> Result<i32> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.y
    } else {
        0
    })
}

pub async fn player_getmap(world: &GameWorld, entity: &crate::Entity) -> Result<MapPosition> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.map
    } else {
        MapPosition::new(0, 0, 0)
    })
}

pub async fn player_gethp(world: &GameWorld, entity: &crate::Entity) -> Result<i32> {
    let lock = world.read().await;
    let mut query = lock.query_one::<&Vitals>(entity.0)?;

    Ok(if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize]
    } else {
        0
    })
}

pub async fn player_setx(world: &GameWorld, entity: &crate::Entity, x: i32) -> Result<()> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.x = x;
    }

    Ok(())
}

pub async fn player_sety(world: &GameWorld, entity: &crate::Entity, y: i32) -> Result<()> {
    let lock = world.write().await;
    let mut query = lock.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.y = y;
    }

    Ok(())
}

pub async fn player_setmap(
    world: &GameWorld,
    entity: &crate::Entity,
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
    entity: &crate::Entity,
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
    entity: &crate::Entity,
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
    entity: &crate::Entity,
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
    entity: &Entity,
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
