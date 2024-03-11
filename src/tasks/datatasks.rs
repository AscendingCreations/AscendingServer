use crate::{
    containers::Storage,
    gametypes::{AscendingError, MapPosition, Result, ServerPackets},
    socket::*,
};

use indexmap::map::Entry;
/* Information Packet Data Portion Worse case is 1400 bytes
* This means you can fit based on Packet Size: 8bytes + Packet ID: 4bytes  + Data array count: 4bytes
this leaves you with 1384 bytes to play with per packet.
* Item Size of 17 bytes can send up to 81 per packet.
* Npc Size 80 bytes can send up to 17 per packet.
* player Size 226 bytes can send up to 6 per packet.
*/

//Token uses the Maps position to Store in the IndexMap.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum DataTaskToken {
    NpcMove(MapPosition),
    NpcDir(MapPosition),
    NpcDeath(MapPosition),
    NpcUnload(MapPosition),
    NpcAttack(MapPosition),
    NpcSpawn(MapPosition),
    NpcVitals(MapPosition),
    NpcDamage(MapPosition),
    PlayerMove(MapPosition),
    PlayerWarp(MapPosition),
    PlayerDir(MapPosition),
    PlayerDeath(MapPosition),
    PlayerUnload(MapPosition),
    PlayerAttack(MapPosition),
    PlayerSpawn(MapPosition),
    PlayerLevel(MapPosition),
    PlayerVitals(MapPosition),
    PlayerDamage(MapPosition),
    MapChat(MapPosition),
    ItemUnload(MapPosition),
    ItemLoad(MapPosition),
    GlobalChat,
}

impl DataTaskToken {
    pub fn add_task<T: ByteBufferWrite>(self, storage: &Storage, data: &T) -> Result<()> {
        match storage.map_cache.borrow_mut().entry(self) {
            Entry::Vacant(v) => {
                let mut buffer = new_cache(self.packet_id())?;
                data.write_to_buffer(&mut buffer)?;
                v.insert(vec![(1, buffer, false)]);
            }
            Entry::Occupied(mut o) => {
                let buffers = o.get_mut();

                if buffers.is_empty() {
                    let mut buffer = new_cache(self.packet_id())?;
                    //write the data into the packet.
                    data.write_to_buffer(&mut buffer)?;
                    //push it to the buffer list.
                    buffers.push((1, buffer, false));
                } else {
                    let (count, buffer, is_finished) = buffers
                        .last_mut()
                        .ok_or(AscendingError::PacketCacheNotFound(self))?;
                    data.write_to_buffer(buffer)?;
                    *count += 1;

                    // If buffer is full lets make another one thats empty.
                    // Also lets Finish the old buffer by adding the count.
                    // We will use the count to deturmine if we send the packet or not.
                    if *count >= self.limits() && !*is_finished {
                        *is_finished = true;
                        finish_cache(buffer, *count, false)?;
                        buffers.push((0, new_cache(self.packet_id())?, false));
                    }
                }
            }
        }

        storage.map_cache_ids.borrow_mut().insert(self);

        Ok(())
    }

    //This is the amount of items per packet being sent limit. this is based on
    //the max empty space 1384 bytes of usable data in the packet.
    pub fn limits(&self) -> u32 {
        match self {
            DataTaskToken::NpcMove(_) | DataTaskToken::PlayerMove(_) => 40,
            DataTaskToken::PlayerWarp(_) => 40,
            DataTaskToken::NpcDir(_)
            | DataTaskToken::PlayerDir(_)
            | DataTaskToken::NpcDeath(_)
            | DataTaskToken::PlayerDeath(_) => 153,
            DataTaskToken::NpcUnload(_)
            | DataTaskToken::PlayerUnload(_)
            | DataTaskToken::NpcAttack(_)
            | DataTaskToken::PlayerAttack(_)
            | DataTaskToken::ItemUnload(_) => 173,
            DataTaskToken::PlayerLevel(_) => 69,
            DataTaskToken::NpcDamage(_) | DataTaskToken::PlayerDamage(_) => 86,
            DataTaskToken::NpcSpawn(_) => 11,
            DataTaskToken::PlayerSpawn(_) => 6,
            DataTaskToken::MapChat(_) | DataTaskToken::GlobalChat => 4, // This one might be more special since it will range heavily.
            DataTaskToken::ItemLoad(_) => 28,
            DataTaskToken::NpcVitals(_) | DataTaskToken::PlayerVitals(_) => 43,
        }
    }

    /// Id of the packet for the data type.
    pub fn packet_id(&self) -> u32 {
        match self {
            DataTaskToken::NpcMove(_) => ServerPackets::NpcMove as u32,
            DataTaskToken::PlayerMove(_) => ServerPackets::PlayerMove as u32,
            DataTaskToken::PlayerWarp(_) => ServerPackets::PlayerWarp as u32,
            DataTaskToken::NpcDir(_) => ServerPackets::NpcDir as u32,
            DataTaskToken::PlayerDir(_) => ServerPackets::PlayerDir as u32,
            DataTaskToken::NpcDeath(_) => ServerPackets::NpcDeath as u32,
            DataTaskToken::PlayerDeath(_) => ServerPackets::PlayerDeath as u32,
            DataTaskToken::NpcUnload(_) => ServerPackets::NpcUnload as u32,
            DataTaskToken::PlayerUnload(_) => ServerPackets::PlayerUnload as u32,
            DataTaskToken::NpcAttack(_) => ServerPackets::NpcAttack as u32,
            DataTaskToken::PlayerAttack(_) => ServerPackets::PlayerAttack as u32,
            DataTaskToken::NpcVitals(_) => ServerPackets::NpcVital as u32,
            DataTaskToken::PlayerVitals(_) => ServerPackets::PlayerVitals as u32,
            DataTaskToken::ItemUnload(_) => ServerPackets::MapItemsUnload as u32,
            DataTaskToken::NpcSpawn(_) => ServerPackets::NpcData as u32,
            DataTaskToken::PlayerSpawn(_) => ServerPackets::PlayerSpawn as u32,
            DataTaskToken::MapChat(_) => ServerPackets::ChatMsg as u32,
            DataTaskToken::GlobalChat => ServerPackets::ChatMsg as u32,
            DataTaskToken::ItemLoad(_) => ServerPackets::MapItems as u32,
            DataTaskToken::NpcDamage(_) => ServerPackets::MapItems as u32, //TODO: Make a packet ID for Damages. This is to display the damage done to a player/npc on hit.
            DataTaskToken::PlayerDamage(_) => ServerPackets::MapItems as u32,
            DataTaskToken::PlayerLevel(_) => ServerPackets::PlayerLevel as u32,
        }
    }

    pub fn send(&self, world: &mut hecs::World, storage: &Storage, buf: ByteBuffer) {
        match self {
            DataTaskToken::GlobalChat => send_to_all(world, storage, buf),
            DataTaskToken::NpcMove(mappos)
            | DataTaskToken::PlayerMove(mappos)
            | DataTaskToken::PlayerWarp(mappos)
            | DataTaskToken::NpcDir(mappos)
            | DataTaskToken::PlayerDir(mappos)
            | DataTaskToken::NpcDeath(mappos)
            | DataTaskToken::PlayerDeath(mappos)
            | DataTaskToken::NpcUnload(mappos)
            | DataTaskToken::PlayerUnload(mappos)
            | DataTaskToken::NpcAttack(mappos)
            | DataTaskToken::PlayerAttack(mappos)
            | DataTaskToken::ItemUnload(mappos)
            | DataTaskToken::NpcSpawn(mappos)
            | DataTaskToken::PlayerSpawn(mappos)
            | DataTaskToken::MapChat(mappos)
            | DataTaskToken::ItemLoad(mappos)
            | DataTaskToken::PlayerVitals(mappos)
            | DataTaskToken::PlayerLevel(mappos)
            | DataTaskToken::PlayerDamage(mappos)
            | DataTaskToken::NpcDamage(mappos)
            | DataTaskToken::NpcVitals(mappos) => send_to_maps(world, storage, *mappos, buf, None),
        }
    }
}

pub fn new_cache(packet_id: u32) -> Result<ByteBuffer> {
    let mut buffer = ByteBuffer::new_packet_with(1412)?;
    //prelocate space for count and packetID
    buffer.write(packet_id)?;
    //preallocate space for count.
    buffer.write(0u32)?;
    Ok(buffer)
}

pub fn finish_cache(buffer: &mut ByteBuffer, count: u32, is_finished: bool) -> Result<()> {
    if !is_finished {
        //Move it 8 bytes for Size + 4 bytes for Packet ID to get count location.
        buffer.move_cursor(12)?;
        //Write the count from the offset cursor position.
        //This will overwrite old data which in this case is empty.
        buffer.write(count)?;
        //finish the buffer off. This sets the Packet size and makes sure the cursor is
        //back to zero again.
        buffer.finish()?;
    }
    Ok(())
}
