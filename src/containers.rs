mod bases;
mod storage;

pub use bases::*;
pub use storage::*;

// Salt used for encrypting passwords within database.
pub const SALT: &[u8] = b"ThisIsMySalt";
