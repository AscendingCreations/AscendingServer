use bytey::{ByteBufferRead, ByteBufferWrite};
use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    ByteBufferRead,
    ByteBufferWrite,
    MByteBufferRead,
    MByteBufferWrite,
    Hash,
)]
pub enum ServerToServerPackets {
    OnlineCheck,
    Verification,
    KillClient,
}

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    ByteBufferRead,
    ByteBufferWrite,
    Hash,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub enum GameServerPacket {
    OnlineCheck,
    Verification,
    UpdateInfo,
    UpdateCount,
}
