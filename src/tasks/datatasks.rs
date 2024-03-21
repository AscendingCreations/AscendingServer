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
    NpcWarp(MapPosition),
    NpcDir(MapPosition),
    NpcDeath(MapPosition),
    NpcAttack(MapPosition),
    NpcSpawn(MapPosition),
    NpcVitals(MapPosition),
    NpcDamage(MapPosition),
    PlayerMove(MapPosition),
    PlayerWarp(MapPosition),
    PlayerDir(MapPosition),
    PlayerDeath(MapPosition),
    PlayerAttack(MapPosition),
    PlayerSpawn(MapPosition),
    PlayerLevel(MapPosition),
    PlayerVitals(MapPosition),
    PlayerDamage(MapPosition),
    MapChat(MapPosition),
    ItemLoad(MapPosition),
    EntityUnload(MapPosition),
    PlayerSpawnToEntity(usize), //SocketID
    NpcSpawnToEntity(usize),    //SocketID
    ItemLoadToEntity(usize),    //SocketID
    GlobalChat,
}

/// Max size of data a packet can hold before it gets split by the OS.
pub const PACKET_DATA_LIMIT: usize = 1400;

impl DataTaskToken {
    pub fn add_task<T: ByteBufferWrite>(self, storage: &Storage, data: &T) -> Result<()> {
        //Newer packets get pushed to the back.
        match storage.packet_cache.borrow_mut().entry(self) {
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
                    //try to get the size but if its a internal Vec thsi migth be wrong.
                    let size = std::mem::size_of::<T>();
                    let (count, buffer, is_finished) = buffers
                        .back_mut()
                        .ok_or(AscendingError::PacketCacheNotFound(self))?;

                    //build a initial packet to get the true size.
                    let mut packet = ByteBuffer::with_capacity(size)?;
                    data.write_to_buffer(&mut packet)?;

                    if packet.length() + buffer.length() > PACKET_DATA_LIMIT {
                        *is_finished = true;
                        finish_cache(buffer, *count, false)?;

                        let mut buffer = new_cache(self.packet_id())?;

                        buffer.write_slice(packet.as_slice())?;

                        if buffer.length() > PACKET_DATA_LIMIT {
                            warn!(
                                "Buffer Length for single write of {:?} Exceeded PACKET_DATA_LIMIT",
                                self
                            );
                        }
                        buffers.push_back((1, buffer, false));
                    } else {
                        buffer.write_slice(packet.as_slice())?;
                        *count += 1;
                    }
                }
            }
        }

        storage.packet_cache_ids.borrow_mut().insert(self);

        Ok(())
    }

    /// Id of the packet for the data type.
    pub fn packet_id(&self) -> ServerPackets {
        use DataTaskToken::*;
        match self {
            NpcMove(_) => ServerPackets::NpcMove,
            NpcWarp(_) => ServerPackets::NpcWarp,
            PlayerMove(_) => ServerPackets::PlayerMove,
            PlayerWarp(_) => ServerPackets::PlayerWarp,
            NpcDir(_) => ServerPackets::NpcDir,
            PlayerDir(_) => ServerPackets::PlayerDir,
            NpcDeath(_) => ServerPackets::NpcDeath,
            PlayerDeath(_) => ServerPackets::PlayerDeath,
            NpcAttack(_) => ServerPackets::NpcAttack,
            PlayerAttack(_) => ServerPackets::PlayerAttack,
            NpcVitals(_) => ServerPackets::NpcVital,
            PlayerVitals(_) => ServerPackets::PlayerVitals,
            EntityUnload(_) => ServerPackets::EntityUnload,
            NpcSpawn(_) | NpcSpawnToEntity(_) => ServerPackets::NpcData,
            PlayerSpawn(_) | PlayerSpawnToEntity(_) => ServerPackets::PlayerSpawn,
            MapChat(_) => ServerPackets::ChatMsg,
            GlobalChat => ServerPackets::ChatMsg,
            ItemLoad(_) | ItemLoadToEntity(_) => ServerPackets::MapItems,
            NpcDamage(_) => ServerPackets::MapItems, //TODO: Make a packet ID for Damages. This is to display the damage done to a player/npc on hit.
            PlayerDamage(_) => ServerPackets::MapItems,
            PlayerLevel(_) => ServerPackets::PlayerLevel,
        }
    }

    pub fn send(&self, world: &mut World, storage: &Storage, buf: ByteBuffer) -> Result<()> {
        use DataTaskToken::*;
        match self {
            GlobalChat => send_to_all(world, storage, buf),
            NpcMove(mappos) | NpcWarp(mappos) | PlayerMove(mappos) | PlayerWarp(mappos)
            | PlayerDir(mappos) | NpcDeath(mappos) | NpcDir(mappos) | PlayerDeath(mappos)
            | EntityUnload(mappos) | NpcAttack(mappos) | PlayerAttack(mappos)
            | NpcSpawn(mappos) | PlayerSpawn(mappos) | MapChat(mappos) | ItemLoad(mappos)
            | PlayerVitals(mappos) | PlayerLevel(mappos) | PlayerDamage(mappos)
            | NpcDamage(mappos) | NpcVitals(mappos) => {
                send_to_maps(world, storage, *mappos, buf, None)
            }
            PlayerSpawnToEntity(socket_id)
            | NpcSpawnToEntity(socket_id)
            | ItemLoadToEntity(socket_id) => send_to(storage, *socket_id, buf),
        }

        Ok(())
    }
}

pub fn process_tasks(world: &mut World, storage: &Storage) -> Result<()> {
    while let Some(id) = storage.packet_cache_ids.borrow_mut().pop() {
        if let Some(buffers) = storage.packet_cache.borrow_mut().get_mut(&id) {
            //We send the older packets first hence pop front as they are the oldest.
            while let Some((count, mut buffer, is_finished)) = buffers.pop_front() {
                finish_cache(&mut buffer, count, is_finished)?;
                id.send(world, storage, buffer)?;
            }

            //lets resize these if they get to unruly.
            if buffers.capacity() > 100 && buffers.len() < 50 {
                buffers.shrink_to_fit()
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
        //Move it 8 bytes for Size + 2 bytes for Packet ID enum to get count location.
        buffer.move_cursor(10)?;
        //Write the count from the offset cursor position.
        //This will overwrite old data which in this case is empty.
        buffer.write(count)?;
        //finish the buffer off. This sets the Packet size and makes sure the cursor is
        //back to zero again.
        buffer.finish()?;
    }
    Ok(())
}
