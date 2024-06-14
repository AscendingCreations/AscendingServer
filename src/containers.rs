mod bases;
mod storage;

pub use bases::*;
pub use storage::*;

//We redefine these here so it is easier to update the hash style later if we need too.
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;
pub type IndexSet<T> = indexmap::IndexSet<T, ahash::RandomState>;
pub type HashSet<T> = std::collections::HashSet<T, ahash::RandomState>;
pub type HashMap<K, V> = std::collections::HashMap<K, V, ahash::RandomState>;

// Salt used for encrypting passwords within database.
pub const SALT: &[u8] = b"ThisIsMySalt";
