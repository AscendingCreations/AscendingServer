use mmap_bytey::{MByteBufferRead, MByteBufferWrite};
use serde::{Deserialize, Serialize};
use speedy::{Readable, Writable};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    Default,
    Readable,
    Writable,
    MByteBufferRead,
    MByteBufferWrite,
)]
pub struct TileBox {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
}
