mod actor;
mod keys;
mod packets;

pub use actor::*;
pub use keys::{ClaimsKey, GlobalKey};
pub use packets::*;

pub type SlotMap<T> = slotmap::SlotMap<GlobalKey, T>;
pub type HopSlotMap<T> = slotmap::HopSlotMap<GlobalKey, T>;
