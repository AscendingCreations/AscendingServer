use crate::{containers::Storage, gametypes::MapPosition, tasks::*};
use indexmap::map::Entry;

/* Information Packet Data Portion Worse case is 1420 bytes
* This means you can fit based on Quantity + 4 byte token header  + 4 bytes for count
* Item Size of 17 bytes can send up to 82 per packet.
* Npc Size 80 bytes can send up to 16 per packet.
* player Size 226 bytes can send up to 5 per packet.
*/

//Buffer Types per map ID, Used to buffer packets together.
#[derive(Clone, Debug)]
pub enum DataTasks {
    NpcMove(Vec<MovePacket>),
    NpcDir(Vec<DirPacket>),
    NpcDeath(Vec<DeathPacket>),
    NpcUnload(Vec<u64>),
    NpcAttack(Vec<u64>),
    NpcSpawn(Vec<NpcSpawnPacket>),
    PlayerMove(Vec<MovePacket>),
    PlayerDir(Vec<DirPacket>),
    PlayerDeath(Vec<DeathPacket>),
    PlayerUnload(Vec<u64>),
    PlayerAttack(Vec<u64>),
    PlayerSpawn(Vec<PlayerSpawnPacket>),
    MapChat(Vec<MessagePacket>),
    ItemUnload(Vec<u64>),
    ItemLoad(Vec<MapItemPacket>),
}

impl DataTasks {
    // Limits on how many can send per packet.
    pub fn limits(&self) -> usize {
        use crate::tasks::DataTasks::*;
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

    pub fn from_data(task_data: TaskData) -> DataTasks {
        match task_data {
            TaskData::NpcMove(data) => {
                let vec = vec![data];
                DataTasks::NpcMove(vec)
            }
            TaskData::NpcDir(data) => {
                let vec = vec![data];
                DataTasks::NpcDir(vec)
            }
            TaskData::NpcDeath(data) => {
                let vec = vec![data];
                DataTasks::NpcDeath(vec)
            }
            TaskData::NpcUnload(data) => {
                let vec = vec![data];
                DataTasks::NpcUnload(vec)
            }
            TaskData::NpcAttack(data) => {
                let vec = vec![data];
                DataTasks::NpcAttack(vec)
            }
            TaskData::NpcSpawn(data) => {
                let vec = vec![data];
                DataTasks::NpcSpawn(vec)
            }
            TaskData::PlayerMove(data) => {
                let vec = vec![data];
                DataTasks::PlayerMove(vec)
            }
            TaskData::PlayerDir(data) => {
                let vec = vec![data];
                DataTasks::PlayerDir(vec)
            }
            TaskData::PlayerDeath(data) => {
                let vec = vec![data];
                DataTasks::PlayerDeath(vec)
            }
            TaskData::PlayerUnload(data) => {
                let vec = vec![data];
                DataTasks::PlayerUnload(vec)
            }
            TaskData::PlayerAttack(data) => {
                let vec = vec![data];
                DataTasks::PlayerAttack(vec)
            }
            TaskData::PlayerSpawn(data) => {
                let vec = vec![data];
                DataTasks::PlayerSpawn(vec)
            }
            TaskData::MapChat(data) => {
                let vec = vec![data];
                DataTasks::MapChat(vec)
            }
            TaskData::ItemUnload(data) => {
                let vec = vec![data];
                DataTasks::ItemUnload(vec)
            }
            TaskData::ItemLoad(data) => {
                let vec = vec![data];
                DataTasks::ItemLoad(vec)
            }
        }
    }
    pub fn push(&mut self, task_data: TaskData) {
        match (task_data, self) {
            (TaskData::NpcMove(value), DataTasks::NpcMove(vec)) => {
                vec.push(value);
            }
            (TaskData::NpcDir(value), DataTasks::NpcDir(vec)) => {
                vec.push(value);
            }
            (TaskData::NpcDeath(value), DataTasks::NpcDeath(vec)) => {
                vec.push(value);
            }
            (TaskData::NpcUnload(value), DataTasks::NpcUnload(vec)) => {
                vec.push(value);
            }
            (TaskData::NpcAttack(value), DataTasks::NpcAttack(vec)) => {
                vec.push(value);
            }
            (TaskData::NpcSpawn(value), DataTasks::NpcSpawn(vec)) => {
                vec.push(value);
            }
            (TaskData::PlayerMove(value), DataTasks::PlayerMove(vec)) => {
                vec.push(value);
            }
            (TaskData::PlayerDir(value), DataTasks::PlayerDir(vec)) => {
                vec.push(value);
            }
            (TaskData::PlayerDeath(value), DataTasks::PlayerDeath(vec)) => {
                vec.push(value);
            }
            (TaskData::PlayerUnload(value), DataTasks::PlayerUnload(vec)) => {
                vec.push(value);
            }
            (TaskData::PlayerAttack(value), DataTasks::PlayerAttack(vec)) => {
                vec.push(value);
            }
            (TaskData::PlayerSpawn(value), DataTasks::PlayerSpawn(vec)) => {
                vec.push(value);
            }
            (TaskData::MapChat(value), DataTasks::MapChat(vec)) => {
                vec.push(value);
            }
            (TaskData::ItemUnload(value), DataTasks::ItemUnload(vec)) => {
                vec.push(value);
            }
            (TaskData::ItemLoad(value), DataTasks::ItemLoad(vec)) => {
                vec.push(value);
            }
            (_, _) => {}
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

impl DataTaskToken {
    pub fn from_data(data: &TaskData, map_pos: MapPosition) -> DataTaskToken {
        match data {
            TaskData::NpcMove(_) => DataTaskToken::NpcMove(map_pos),
            TaskData::NpcDir(_) => DataTaskToken::NpcDir(map_pos),
            TaskData::NpcDeath(_) => DataTaskToken::NpcDeath(map_pos),
            TaskData::NpcUnload(_) => DataTaskToken::NpcUnload(map_pos),
            TaskData::NpcAttack(_) => DataTaskToken::NpcAttack(map_pos),
            TaskData::NpcSpawn(_) => DataTaskToken::NpcSpawn(map_pos),
            TaskData::PlayerMove(_) => DataTaskToken::PlayerMove(map_pos),
            TaskData::PlayerDir(_) => DataTaskToken::PlayerDir(map_pos),
            TaskData::PlayerDeath(_) => DataTaskToken::PlayerDeath(map_pos),
            TaskData::PlayerUnload(_) => DataTaskToken::PlayerUnload(map_pos),
            TaskData::PlayerAttack(_) => DataTaskToken::PlayerAttack(map_pos),
            TaskData::PlayerSpawn(_) => DataTaskToken::PlayerSpawn(map_pos),
            TaskData::MapChat(_) => DataTaskToken::MapChat(map_pos),
            TaskData::ItemUnload(_) => DataTaskToken::ItemUnload(map_pos),
            TaskData::ItemLoad(_) => DataTaskToken::ItemLoad(map_pos),
        }
    }
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

impl TaskData {
    pub fn add_task(self, world: &Storage, map_pos: MapPosition) {
        let key = DataTaskToken::from_data(&self, map_pos);
        match world.map_data_tasks.borrow_mut().entry(key) {
            Entry::Vacant(v) => {
                let datatasks = DataTasks::from_data(self);
                v.insert(datatasks);
            }
            Entry::Occupied(mut o) => {
                o.get_mut().push(self);
            }
        }
    }
}
