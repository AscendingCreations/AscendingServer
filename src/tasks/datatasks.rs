use crate::{
    containers::Storage,
    gametypes::{AscendingError, MapPosition, Result},
    socket::*,
};
use hecs::World;
use indexmap::map::Entry;
use log::warn;
use mmap_bytey::BUFFER_SIZE;
use std::collections::VecDeque;
/* Information Packet Data Portion Worse case is 1400 bytes
* This means you can fit based on Packet Size: 8bytes + Packet ID: 4bytes  + Data array count: 4bytes
*this leaves you with 1384 bytes to play with per packet.
*/

//Token uses the Maps position to Store in the IndexMap.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum DataTaskToken {
    Move(MapPosition),
    Warp(MapPosition),
    Dir(MapPosition),
    Death(MapPosition),
    Attack(MapPosition),
    NpcSpawn(MapPosition),
    Damage(MapPosition),
    PlayerSpawn(MapPosition),
    PlayerLevel(MapPosition),
    Vitals(MapPosition),
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
    pub async fn add_task(self, storage: &Storage, mut data: MByteBuffer) -> Result<()> {
        //Newer packets get pushed to the back.
        match storage.packet_cache.lock().await.entry(self) {
            Entry::Vacant(v) => {
                let mut buffer = new_cache(self.packet_id())?;
                buffer.write_slice(data.as_slice())?;
                v.insert(VecDeque::from_iter([(1, buffer, false)]));
            }
            Entry::Occupied(mut o) => {
                let buffers = o.get_mut();

                if buffers.is_empty() {
                    let mut buffer = new_cache(self.packet_id())?;
                    buffer.write_slice(data.as_slice())?;
                    buffers.push_back((1, buffer, false));
                } else {
                    let (count, buffer, is_finished) = buffers
                        .back_mut()
                        .ok_or(AscendingError::PacketCacheNotFound(self))?;

                    if data.length() + buffer.length() > BUFFER_SIZE {
                        *is_finished = true;
                        finish_cache(buffer, *count, false)?;

                        let mut buffer = new_cache(self.packet_id())?;

                        buffer.write_slice(data.as_slice())?;
                        buffers.push_back((1, buffer, false));
                    } else {
                        buffer.write_slice(data.as_slice())?;
                        *count += 1;
                    }
                }
            }
        }

        storage.packet_cache_ids.lock().await.insert(self);

        Ok(())
    }

    /// Id of the packet for the data type.
    pub fn packet_id(&self) -> ServerPackets {
        use DataTaskToken::*;
        match self {
            Move(_) => ServerPackets::Move,
            Warp(_) => ServerPackets::Warp,
            Dir(_) => ServerPackets::Dir,
            Death(_) => ServerPackets::Death,
            Attack(_) => ServerPackets::Attack,
            Vitals(_) => ServerPackets::Vitals,
            EntityUnload(_) => ServerPackets::EntityUnload,
            NpcSpawn(_) | NpcSpawnToEntity(_) => ServerPackets::NpcData,
            PlayerSpawn(_) | PlayerSpawnToEntity(_) => ServerPackets::PlayerSpawn,
            MapChat(_) => ServerPackets::ChatMsg,
            GlobalChat => ServerPackets::ChatMsg,
            ItemLoad(_) | ItemLoadToEntity(_) => ServerPackets::MapItems,
            Damage(_) => ServerPackets::Damage,
            PlayerLevel(_) => ServerPackets::PlayerLevel,
        }
    }

    pub async fn send(&self, world: &mut World, storage: &Storage, buf: MByteBuffer) -> Result<()> {
        use DataTaskToken::*;
        match self {
            GlobalChat => send_to_all(world, storage, buf).await,
            Move(mappos) | Warp(mappos) | Death(mappos) | Dir(mappos) | EntityUnload(mappos)
            | Attack(mappos) | NpcSpawn(mappos) | PlayerSpawn(mappos) | MapChat(mappos)
            | ItemLoad(mappos) | Vitals(mappos) | PlayerLevel(mappos) | Damage(mappos) => {
                send_to_maps(world, storage, *mappos, buf, None).await
            }
            PlayerSpawnToEntity(socket_id)
            | NpcSpawnToEntity(socket_id)
            | ItemLoadToEntity(socket_id) => send_to(storage, *socket_id, buf).await,
        }
    }
}

pub async fn process_tasks(world: &mut World, storage: &Storage) -> Result<()> {
    for id in storage.packet_cache_ids.lock().await.drain(..) {
        if let Some(buffers) = storage.packet_cache.lock().await.get_mut(&id) {
            //We send the older packets first hence pop front as they are the oldest.
            for (count, mut buffer, is_finished) in buffers.drain(..) {
                finish_cache(&mut buffer, count, is_finished)?;
                id.send(world, storage, buffer).await?;
            }

            //lets resize these if they get to unruly.
            if buffers.capacity() > 250 && buffers.len() < 100 {
                warn!(
                    "process_tasks: packet_cache Buffer Strink to 100, Current Capacity {}, Current len {}.",
                    buffers.capacity(),
                    buffers.len()
                );
                buffers.shrink_to(100);
            }
        }
    }

    Ok(())
}

pub fn new_cache(packet_id: ServerPackets) -> Result<MByteBuffer> {
    //Set it to the max packet size - the size holder - packet_id - count
    let mut buffer = MByteBuffer::new_packet()?;
    //Write the packet ID so we know where it goes.
    buffer.write(packet_id)?;
    //preallocate space for count.
    buffer.write(0u32)?;
    Ok(buffer)
}

pub fn finish_cache(buffer: &mut MByteBuffer, count: u32, is_finished: bool) -> Result<()> {
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
