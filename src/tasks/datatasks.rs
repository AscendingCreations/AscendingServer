use crate::{containers::*, gametypes::*, maps::MapActor, network::*, GlobalKey};
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
    PlayerSpawnToEntity(GlobalKey),
    NpcSpawnToEntity(GlobalKey),
    ItemLoadToEntity(GlobalKey),
    GlobalChat,
}

/// Max size of data a packet can hold before it gets split by the OS.
pub const PACKET_DATA_LIMIT: usize = 1400;

impl DataTaskToken {
    pub async fn add_task(self, map: &mut MapActor, mut data: MByteBuffer) -> Result<()> {
        //Newer packets get pushed to the back.
        match map.packet_cache.entry(self) {
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

        map.packet_cache_ids.insert(self);

        Ok(())
    }

    /// Id of the packet for the data type.
    pub fn packet_id(&self) -> ServerPacketID {
        use DataTaskToken::*;
        match self {
            Move(_) => ServerPacketID::Move,
            Warp(_) => ServerPacketID::Warp,
            Dir(_) => ServerPacketID::Dir,
            Death(_) => ServerPacketID::Death,
            Attack(_) => ServerPacketID::Attack,
            Vitals(_) => ServerPacketID::Vitals,
            EntityUnload(_) => ServerPacketID::EntityUnload,
            NpcSpawn(_) | NpcSpawnToEntity(_) => ServerPacketID::NpcData,
            PlayerSpawn(_) | PlayerSpawnToEntity(_) => ServerPacketID::PlayerSpawn,
            MapChat(_) => ServerPacketID::ChatMsg,
            GlobalChat => ServerPacketID::ChatMsg,
            ItemLoad(_) | ItemLoadToEntity(_) => ServerPacketID::MapItems,
            Damage(_) => ServerPacketID::Damage,
            PlayerLevel(_) => ServerPacketID::PlayerLevel,
        }
    }

    pub async fn send(&self, map: &mut MapActor, buf: MByteBuffer) -> Result<()> {
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

pub async fn process_tasks(map: &mut MapActor) -> Result<()> {
    let mut sends = Vec::with_capacity(128);

    for id in map.packet_cache_ids.drain(..) {
        if let Some(buffers) = map.packet_cache.get_mut(&id) {
            for (count, mut buffer, is_finished) in buffers.drain(..) {
                finish_cache(&mut buffer, count, is_finished)?;
                sends.push((id, buffer));
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

    for (id, buffer) in sends.into_iter() {
        id.send(map, buffer).await?;
    }

    Ok(())
}

pub fn new_cache(packet_id: ServerPacketID) -> Result<MByteBuffer> {
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
