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
    pub fn add_task<T: ToBuffer>(self, world: &Storage, data: &T) -> Result<DataTaskToken> {
        let mut buffer = bytey::ByteBuffer::with_capacity(data.buffer_size())?;
        data.to_buffer(&mut buffer)?;

        match world.map_cache.borrow_mut().entry(self) {
            Entry::Vacant(v) => {
                v.insert(vec![buffer]);
            }
            Entry::Occupied(mut o) => {
                o.get_mut().push(buffer);
            }
        }

        Ok(self)
    }
}
