use crate::{
    containers::Storage,
    gametypes::{AscendingError, MapPosition, Result, ServerPackets},
    socket::*,
};
use hecs::World;
use indexmap::map::Entry;
use log::warn;
use std::collections::VecDeque;
/* Information Packet Data Portion Worse case is 1400 bytes
* This means you can fit based on Packet Size: 8bytes + Packet ID: 4bytes  + Data array count: 4bytes
*this leaves you with 1384 bytes to play with per packet.
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

/// Max size of data a packet can hold before it gets split by the OS.
pub const PACKET_DATA_LIMIT: usize = 1400;

impl DataTaskToken {
    pub fn add_task<T: ByteBufferWrite>(self, storage: &Storage, data: &T) -> Result<()> {
        //Newer packets get pushed to the back.
        match storage.map_cache.borrow_mut().entry(self) {
            Entry::Vacant(v) => {
                let mut buffer = new_cache(self.packet_id())?;
                data.write_to_buffer(&mut buffer)?;
                if buffer.length() > PACKET_DATA_LIMIT {
                    warn!(
                        "Buffer Length for single write of {:?} Exceeded PACKET_DATA_LIMIT",
                        self
                    );
                }

                v.insert(VecDeque::from_iter([(1, buffer, false)]));
            }
            Entry::Occupied(mut o) => {
                let buffers = o.get_mut();

                if buffers.is_empty() {
                    let mut buffer = new_cache(self.packet_id())?;
                    data.write_to_buffer(&mut buffer)?;
                    buffers.push_back((1, buffer, false));
                } else {
                    let size = std::mem::size_of::<T>();
                    let (count, buffer, is_finished) = buffers
                        .back_mut()
                        .ok_or(AscendingError::PacketCacheNotFound(self))?;

                    if size + buffer.length() > PACKET_DATA_LIMIT {
                        *is_finished = true;
                        finish_cache(buffer, *count, false)?;

                        let mut buffer = new_cache(self.packet_id())?;

                        data.write_to_buffer(&mut buffer)?;

                        if buffer.length() > PACKET_DATA_LIMIT {
                            warn!(
                                "Buffer Length for single write of {:?} Exceeded PACKET_DATA_LIMIT",
                                self
                            );
                        }
                        buffers.push_back((1, buffer, false));
                    } else {
                        data.write_to_buffer(buffer)?;
                        *count += 1;
                    }
                }
            }
        }

        storage.map_cache_ids.borrow_mut().insert(self);

        Ok(())
    }

    /// Id of the packet for the data type.
    pub fn packet_id(&self) -> ServerPackets {
        use DataTaskToken::*;
        match self {
            NpcMove(_) => ServerPackets::NpcMove,
            PlayerMove(_) => ServerPackets::PlayerMove,
            PlayerWarp(_) => ServerPackets::PlayerWarp,
            NpcDir(_) => ServerPackets::NpcDir,
            PlayerDir(_) => ServerPackets::PlayerDir,
            NpcDeath(_) => ServerPackets::NpcDeath,
            PlayerDeath(_) => ServerPackets::PlayerDeath,
            NpcUnload(_) => ServerPackets::NpcUnload,
            PlayerUnload(_) => ServerPackets::PlayerUnload,
            NpcAttack(_) => ServerPackets::NpcAttack,
            PlayerAttack(_) => ServerPackets::PlayerAttack,
            NpcVitals(_) => ServerPackets::NpcVital,
            PlayerVitals(_) => ServerPackets::PlayerVitals,
            ItemUnload(_) => ServerPackets::MapItemsUnload,
            NpcSpawn(_) => ServerPackets::NpcData,
            PlayerSpawn(_) => ServerPackets::PlayerSpawn,
            MapChat(_) => ServerPackets::ChatMsg,
            GlobalChat => ServerPackets::ChatMsg,
            ItemLoad(_) => ServerPackets::MapItems,
            NpcDamage(_) => ServerPackets::MapItems, //TODO: Make a packet ID for Damages. This is to display the damage done to a player/npc on hit.
            PlayerDamage(_) => ServerPackets::MapItems,
            PlayerLevel(_) => ServerPackets::PlayerLevel,
        }
    }

    pub fn send(&self, world: &mut World, storage: &Storage, buf: ByteBuffer) -> Result<()> {
        use DataTaskToken::*;
        match self {
            GlobalChat => send_to_all(world, storage, buf),
            NpcMove(mappos) | PlayerMove(mappos) | PlayerWarp(mappos) | NpcDir(mappos)
            | PlayerDir(mappos) | NpcDeath(mappos) | PlayerDeath(mappos) | NpcUnload(mappos)
            | PlayerUnload(mappos) | NpcAttack(mappos) | PlayerAttack(mappos)
            | ItemUnload(mappos) | NpcSpawn(mappos) | PlayerSpawn(mappos) | MapChat(mappos)
            | ItemLoad(mappos) | PlayerVitals(mappos) | PlayerLevel(mappos)
            | PlayerDamage(mappos) | NpcDamage(mappos) | NpcVitals(mappos) => {
                send_to_maps(world, storage, *mappos, buf, None)
            }
        }

        Ok(())
    }
}

pub fn process_tasks(world: &mut World, storage: &Storage) -> Result<()> {
    while let Some(id) = storage.map_cache_ids.borrow_mut().pop() {
        if let Some(buffers) = storage.map_cache.borrow_mut().get_mut(&id) {
            //We send the older packets first hence pop front as they are the oldest.
            while let Some((count, mut buffer, is_finished)) = buffers.pop_front() {
                finish_cache(&mut buffer, count, is_finished)?;
                id.send(world, storage, buffer)?;
            }
        }
    }

    Ok(())
}

pub fn new_cache(packet_id: ServerPackets) -> Result<ByteBuffer> {
    //Set it to the max packet size - the size holder - packet_id - count
    let mut buffer = ByteBuffer::new_packet_with(PACKET_DATA_LIMIT - 8)?;
    //Write the packet ID so we know where it goes.
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
