use crate::{
    containers::Storage,
    gametypes::{MapPosition, Result},
    socket::*,
};

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

    /// Amount of packets per each packet for this type.
    /// Remember the smallest size is 1404 bytes.
    /// After we already use 16bytes for intenral data needs
    fn limit(&self) -> usize;
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
        match world.map_cache.borrow_mut().entry(self) {
            Entry::Vacant(v) => {
                let mut buffer = ByteBuffer::new_packet_with(1412)?;
                data.to_buffer(&mut buffer)?;
                v.insert(vec![(1, buffer)]);
            }
            Entry::Occupied(mut o) => {
                let buffers = o.get_mut();

                if buffers.is_empty() {
                    let mut buffer = ByteBuffer::new_packet_with(1412)?;
                    //prelocate space for count and packetID
                    buffer.write(0u64)?;
                    data.to_buffer(&mut buffer)?;
                    buffers.push((1, buffer));
                } else {
                    let mut buffer = buffers.last_mut().unwrap();
                    data.to_buffer(&mut buffer.1)?;
                    buffer.0 += 1;

                    // if buffer is full lets make another one thats empty.
                    if buffer.0 >= data.limit() {
                        let mut buffer = ByteBuffer::new_packet_with(1412)?;
                        //prelocate space for count and packetID
                        buffer.write(0u64)?;
                        buffers.push((0, buffer));
                    }
                }
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