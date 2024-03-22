use crate::{
    containers::Storage,
    gametypes::*,
    items::Item,
    tasks::{DataTaskToken, MapItemPacket},
    time_ext::MyInstant,
};
use bytey::{ByteBufferRead, ByteBufferWrite};
use hecs::World;

use super::create_mapitem;

#[derive(Copy, Clone, PartialEq, Eq, Default, ByteBufferRead, ByteBufferWrite)]
pub struct MapItem {
    pub item: Item,
    #[bytey(skip)]
    pub despawn: Option<MyInstant>,
    #[bytey(skip)]
    pub ownertimer: Option<MyInstant>,
    #[bytey(skip)]
    pub ownerid: Option<Entity>,
    pub pos: Position,
}

impl MapItem {
    #[inline(always)]
    pub fn new(num: u32) -> Self {
        let mut item = MapItem::default();
        item.item.num = num;
        item
    }
}

pub fn try_drop_item(
    world: &mut World,
    storage: &Storage,
    index: u32,
    amount: u16,
    pos: Position,
) -> bool {
    let mut storage_mapitem = storage.map_items.borrow_mut();

    // Find open position
    let set_pos = if !storage_mapitem.contains_key(&pos) {
        let mapdata = storage.maps.get(&pos.map);
        if let Some(map_data) = mapdata {
            if !map_data
                .borrow()
                .is_blocked_tile(pos, WorldEntityType::MapItem)
            {
                Some(pos)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        let mut got_pos = None;
        'endcheck: for x in pos.x - 1..=pos.x + 1 {
            for y in pos.y - 1..=pos.y + 1 {
                let mut check_pos = Position { x, y, ..pos };
                if check_pos.x < 0 {
                    check_pos.x = 31;
                    check_pos.map.x -= 1;
                }
                if check_pos.x >= 32 {
                    check_pos.x = 0;
                    check_pos.map.x += 1;
                }
                if check_pos.y < 0 {
                    check_pos.y = 31;
                    check_pos.map.y -= 1;
                }
                if check_pos.y >= 32 {
                    check_pos.y = 0;
                    check_pos.map.y += 1;
                }

                if !storage_mapitem.contains_key(&check_pos) {
                    let mapdata = storage.maps.get(&check_pos.map);
                    if let Some(map_data) = mapdata {
                        if !map_data
                            .borrow()
                            .is_blocked_tile(check_pos, WorldEntityType::MapItem)
                        {
                            got_pos = Some(check_pos);
                        }
                    }
                    break 'endcheck;
                }
            }
        }
        got_pos
    };

    // If we found position, we spawn the item
    if let Some(got_pos) = set_pos {
        let mapdata = storage.maps.get(&got_pos.map);
        if let Some(map_data) = mapdata {
            let map_item = create_mapitem(index, amount, got_pos);
            let id = world.spawn((WorldEntityType::MapItem, map_item));
            let _ = world.insert_one(id, EntityType::MapItem(Entity(id)));
            storage_mapitem.insert(got_pos, Entity(id));
            let _ = DataTaskToken::ItemLoad(got_pos.map).add_task(
                storage,
                &MapItemPacket::new(
                    Entity(id),
                    map_item.pos,
                    map_item.item,
                    map_item.ownerid,
                    true,
                ),
            );
            map_data.borrow_mut().itemids.insert(Entity(id));
            return true;
        }
    }

    false
}
