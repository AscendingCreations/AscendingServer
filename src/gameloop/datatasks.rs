use crate::{gameloop::*, gametypes::MapPosition, players::*};

/* Information Packet Data Portion Worse case is 1420 bytes
* This means you can fit based on Quantity + 4 byte token header  + 4 bytes for count
* Item Size of 17 bytes can send up to 82 per packet.
* Npc Size 80 bytes can send up to 16 per packet.
* player Size 226 bytes can send up to 5 per packet.
*/

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataTask<T> {
    mapid: MapPosition,
    data: Vec<T>, //Vec to Add, Iter, then Clear.
}

impl<T> DataTask<T> {
    pub fn new(mapid: MapPosition) -> DataTask<T> {
        DataTask::<T> {
            mapid,
            data: Vec::with_capacity(32),
        }
    }
}

//Buffer Types per map ID, Used to buffer packets together.
#[derive(Clone, Debug)]
pub enum DataTasks {
    NpcMove(DataTask<MovePacket>),
    NpcDir(DataTask<DirPacket>),
    NpcDeath(DataTask<DeathPacket>),
    NpcUnload(DataTask<u64>),
    NpcAttack(DataTask<u64>),
    NpcSpawn(DataTask<NpcSpawnPacket>),
    PlayerMove(DataTask<MovePacket>),
    PlayerDir(DataTask<DirPacket>),
    PlayerDeath(DataTask<DeathPacket>),
    PlayerUnload(DataTask<u64>),
    PlayerAttack(DataTask<u64>),
    PlayerSpawn(DataTask<PlayerSpawnPacket>),
    MapChat(DataTask<MessagePacket>),
    ItemUnload(DataTask<u64>),
    ItemLoad(DataTask<MapItemPacket>),
}

impl DataTasks {
    // Limits on how many can send per packet.
    pub fn limits(&self) -> usize {
        use crate::gameloop::DataTasks::*;
        match self {
            NpcMove(_) | PlayerMove(_) => 41,
            NpcDir(_) | PlayerDir(_) | NpcDeath(_) | PlayerDeath(_) => 157,
            NpcUnload(_) | PlayerUnload(_) | NpcAttack(_) | PlayerAttack(_) | ItemUnload(_) => 176,
            NpcSpawn(_) => 16,
            PlayerSpawn(_) => 8,
            MapChat(_) => 4, // This one might be more special since it will range heavily.
            ItemLoad(_) => 28,
        }
    }
}

//Token uses the Maps position to Store in the IndexMap.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskData {
    NpcMove(MovePacket),
    NpcDir(DirPacket),
    NpcDeath(DeathPacket),
    NpcUnload(u64),
    NpcAttack(u64),
    NpcSpawn(NpcSpawnPacket),
    PlayerMove(MovePacket),
    PlayerDir(DirPacket),
    PlayerDeath(DeathPacket),
    PlayerUnload(u64),
    PlayerAttack(u64),
    PlayerSpawn(PlayerSpawnPacket),
    MapChat(MessagePacket),
    ItemUnload(u64),
    ItemLoad(MapItemPacket),
}
