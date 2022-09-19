use crate::{
    containers::Storage,
    gametypes::{MapPosition, Result},
};
use bytey::ByteBuffer;
use std::collections::hash_map::Entry;
/* Information Packet Data Portion Worse case is 1420 bytes
* This means you can fit based on Quantity + 4 byte token header  + 4 bytes for count
* Item Size of 17 bytes can send up to 82 per packet.
* Npc Size 80 bytes can send up to 16 per packet.
* player Size 226 bytes can send up to 5 per packet.
*/

//For Data task translation to a byte buffer.
pub trait ToBuffer {
    fn to_buffer(&self, buffer: &mut ByteBuffer) -> Result<()>;
    fn buffer_size(&self) -> usize;
}

//Token uses the Maps position to Store in the IndexMap.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum DataTaskToken {
    NpcMove(MapPosition),
    NpcDir(MapPosition),
    NpcDeath(MapPosition),
    NpcUnload(MapPosition),
    NpcAttack(MapPosition),
    NpcSpawn(MapPosition),
    PlayerMove(MapPosition),
    PlayerDir(MapPosition),
    PlayerDeath(MapPosition),
    PlayerUnload(MapPosition),
    PlayerAttack(MapPosition),
    PlayerSpawn(MapPosition),
    MapChat(MapPosition),
    ItemUnload(MapPosition),
    ItemLoad(MapPosition),
}

impl DataTaskToken {
    pub fn add_task<T: ToBuffer>(self, world: &Storage, data: &T) -> Result<()> {
        let mut buffer = bytey::ByteBuffer::with_capacity(data.buffer_size())?;
        data.to_buffer(&mut buffer)?;

        //lets ensure the buffer is set to cursor 0 for writes to another buffer.
        unsafe {
            buffer.move_cursor_unchecked(0);
        }

        match world.map_cache.borrow_mut().entry(self) {
            Entry::Vacant(v) => {
                v.insert(vec![buffer]);
            }
            Entry::Occupied(mut o) => {
                o.get_mut().push(buffer);
            }
        }

        world.map_cache_ids.borrow_mut().insert(self);

        Ok(())
    }

    //This is the amount of items per packet being sent limit. this is based on
    //the max empty space 1420 bytes of usable data in the packet. / the overall size -1.
    pub fn limits(&self) -> usize {
        match self {
            DataTaskToken::NpcMove(_) | DataTaskToken::PlayerMove(_) => 41,
            DataTaskToken::NpcDir(_)
            | DataTaskToken::PlayerDir(_)
            | DataTaskToken::NpcDeath(_)
            | DataTaskToken::PlayerDeath(_) => 157,
            DataTaskToken::NpcUnload(_)
            | DataTaskToken::PlayerUnload(_)
            | DataTaskToken::NpcAttack(_)
            | DataTaskToken::PlayerAttack(_)
            | DataTaskToken::ItemUnload(_) => 176,
            DataTaskToken::NpcSpawn(_) => 16,
            DataTaskToken::PlayerSpawn(_) => 8,
            DataTaskToken::MapChat(_) => 4, // This one might be more special since it will range heavily.
            DataTaskToken::ItemLoad(_) => 28,
        }
    }
}
