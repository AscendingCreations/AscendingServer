use crate::{containers::*, gametypes::*, items::*, socket::*, sql::*, tasks::*, time_ext::*};
use educe::Educe;
use hecs::{Bundle, World};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Bundle)]
pub struct Socket {
    // IP address
    pub addr: Arc<String>,
    // Socket ID
    pub id: usize,
    // Packet Buffer
    pub buffer: Arc<Mutex<ByteBuffer>>,
}

impl Socket {
    #[inline(always)]
    pub fn new(id: usize, addr: String) -> Result<Self> {
        Ok(Self {
            id,
            addr: Arc::new(addr),
            buffer: Arc::new(Mutex::new(ByteBuffer::with_capacity(8192)?)),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub username: String,
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
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, ByteBufferRead, ByteBufferWrite,
)]
#[educe(Default)]
pub struct Inventory {
    #[educe(Default = (0..MAX_INV).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, ByteBufferRead, ByteBufferWrite,
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
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, ByteBufferRead, ByteBufferWrite,
)]
#[educe(Default)]
pub struct PlayerStorage {
    #[educe(Default = (0..MAX_STORAGE).map(|_| Item::default()).collect())]
    pub items: Vec<Item>,
}

#[derive(
    PartialEq, Eq, Clone, Debug, Educe, Deserialize, Serialize, ByteBufferRead, ByteBufferWrite,
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

pub fn is_player_online(world: &mut World, entity: &crate::Entity) -> Result<bool> {
    Ok(
        *world.get::<&WorldEntityType>(entity.0)? == WorldEntityType::Player
            && *world.get::<&OnlineType>(entity.0)? == OnlineType::Online,
    )
}

#[inline(always)]
pub fn player_switch_maps(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    new_pos: Position,
) -> Result<(Position, bool)> {
    let old_position = world.get_or_err::<Position>(entity)?;

    if let Some(mapref) = storage.maps.get(&old_position.map) {
        let mut map = mapref.borrow_mut();
        map.remove_player(storage, *entity);
        map.remove_entity_from_grid(old_position);
    } else {
        return Ok((old_position, false));
    }

    if let Some(mapref) = storage.maps.get(&new_pos.map) {
        let mut map = mapref.borrow_mut();
        map.add_player(storage, *entity);
        map.add_entity_to_grid(new_pos);
    } else {
        if let Some(mapref) = storage.maps.get(&old_position.map) {
            let mut map = mapref.borrow_mut();
            map.add_player(storage, *entity);
            map.add_entity_to_grid(old_position);
        }

        return Ok((old_position, false));
    }

    *world.get::<&mut Position>(entity.0)? = new_pos;

    Ok((old_position, true))
}

#[inline(always)]
pub fn player_swap_pos(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    pos: Position,
) -> Result<Position> {
    let mut query = world.query_one::<&mut Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        let old_position = *player_position;

        if old_position != pos {
            *player_position = pos;

            let mut map = match storage.maps.get(&old_position.map) {
                Some(map) => map,
                None => return Ok(old_position),
            }
            .borrow_mut();
            map.remove_entity_from_grid(old_position);
            map.add_entity_to_grid(pos);
        }

        old_position
    } else {
        Position::default()
    })
}

pub fn player_add_up_vital(world: &mut World, entity: &crate::Entity, vital: usize) -> Result<i32> {
    let mut query = world.query_one::<&mut Vitals>(entity.0)?;

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
pub fn player_set_dir(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    dir: u8,
) -> Result<()> {
    let mut query = world.query_one::<(&mut Dir, &Position)>(entity.0)?;

    if let Some((player_dir, player_position)) = query.get() {
        if player_dir.0 != dir {
            player_dir.0 = dir;

            DataTaskToken::PlayerDir(player_position.map)
                .add_task(storage, dir_packet(*entity, dir)?)?;
        }
    }

    Ok(())
}

pub fn player_getx(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    let mut query = world.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.x
    } else {
        0
    })
}

pub fn player_gety(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    let mut query = world.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.y
    } else {
        0
    })
}

pub fn player_getmap(world: &mut World, entity: &crate::Entity) -> Result<MapPosition> {
    let mut query = world.query_one::<&Position>(entity.0)?;

    Ok(if let Some(player_position) = query.get() {
        player_position.map
    } else {
        MapPosition::new(0, 0, 0)
    })
}

pub fn player_gethp(world: &mut World, entity: &crate::Entity) -> Result<i32> {
    let mut query = world.query_one::<&Vitals>(entity.0)?;

    Ok(if let Some(player_vital) = query.get() {
        player_vital.vital[VitalTypes::Hp as usize]
    } else {
        0
    })
}

pub fn player_setx(world: &mut World, entity: &crate::Entity, x: i32) -> Result<()> {
    let mut query = world.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.x = x;
    }

    Ok(())
}

pub fn player_sety(world: &mut World, entity: &crate::Entity, y: i32) -> Result<()> {
    let mut query = world.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.y = y;
    }

    Ok(())
}

pub fn player_setmap(world: &mut World, entity: &crate::Entity, map: MapPosition) -> Result<()> {
    let mut query = world.query_one::<&mut Position>(entity.0)?;

    if let Some(player_position) = query.get() {
        player_position.map = map;
    }

    Ok(())
}

pub fn player_set_vital(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    vital: VitalTypes,
    amount: i32,
) -> Result<()> {
    {
        let mut query = world.query_one::<&mut Vitals>(entity.0)?;

        if let Some(player_vital) = query.get() {
            player_vital.vital[vital as usize] = amount.min(player_vital.vitalmax[vital as usize]);
        }
    }

    DataTaskToken::PlayerVitals(world.get_or_default::<Position>(entity).map).add_task(
        storage,
        {
            let vitals = world.get_or_err::<Vitals>(entity)?;

            vitals_packet(*entity, vitals.vital, vitals.vitalmax)?
        },
    )?;

    Ok(())
}

#[inline]
pub fn player_give_vals(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    amount: u64,
) -> Result<u64> {
    let player_money = world.get_or_err::<Money>(entity)?;
    let rem = u64::MAX.saturating_sub(player_money.vals);

    if rem > 0 {
        let mut cur = amount;
        if rem >= cur {
            {
                world.get::<&mut Money>(entity.0)?.vals =
                    world.get_or_err::<Money>(entity)?.vals.saturating_add(cur);
            }
            cur = 0;
        } else {
            {
                world.get::<&mut Money>(entity.0)?.vals = u64::MAX;
            }
            cur = cur.saturating_sub(rem);
        }

        send_money(world, storage, entity)?;
        update_currency(storage, world, entity)?;
        send_fltalert(
            storage,
            world.get::<&Socket>(entity.0)?.id,
            format!("You Have Received {} Vals.", amount - cur),
            FtlType::Money,
        )?;
        return Ok(cur);
    }

    Ok(amount)
}

#[inline]
pub fn player_take_vals(
    world: &mut World,
    storage: &Storage,
    entity: &crate::Entity,
    amount: u64,
) -> Result<()> {
    let mut cur = amount;

    let player_money = world.get_or_err::<Money>(entity)?;
    if player_money.vals >= cur {
        {
            world.get::<&mut Money>(entity.0)?.vals =
                world.get_or_err::<Money>(entity)?.vals.saturating_sub(cur);
        }
    } else {
        cur = player_money.vals;
        {
            world.get::<&mut Money>(entity.0)?.vals = 0;
        }
    }

    send_money(world, storage, entity)?;
    update_currency(storage, world, entity)?;
    send_fltalert(
        storage,
        world.get::<&Socket>(entity.0)?.id,
        format!("You Lost {} Vals.", cur),
        FtlType::Money,
    )
}

pub fn send_swap_error(
    _world: &mut World,
    storage: &Storage,
    old_socket_id: usize,
    socket_id: usize,
) -> Result<()> {
    send_infomsg(
        storage,
        old_socket_id,
        "Server Error in player swap".into(),
        1,
        true,
    )?;

    send_infomsg(
        storage,
        socket_id,
        "Server Error in player swap".into(),
        1,
        true,
    )
}

pub fn send_login_info(
    world: &mut World,
    storage: &Storage,
    entity: &Entity,
    code: String,
    handshake: String,
    socket_id: usize,
    username: String,
) -> Result<()> {
    world.insert(
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

    storage.player_names.borrow_mut().insert(username, *entity);
    send_myindex(storage, socket_id, entity)?;
    send_codes(world, storage, entity, code, handshake)
}
