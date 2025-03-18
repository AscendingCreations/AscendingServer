mod bases;
mod entity;
mod storage;
mod world;

pub use bases::*;
pub use entity::*;
pub use storage::*;
pub use world::*;

//We redefine these here so it is easier to update the hash style later if we need too.
pub type AHashBuildHasher = std::hash::BuildHasherDefault<ahash::AHasher>;

pub type SparseSecondaryMap<K, V> = slotmap::SparseSecondaryMap<K, V, AHashBuildHasher>;
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, AHashBuildHasher>;
pub type IndexSet<T> = indexmap::IndexSet<T, AHashBuildHasher>;
pub type HashSet<T> = std::collections::HashSet<T, AHashBuildHasher>;
pub type HashMap<K, V> = std::collections::HashMap<K, V, AHashBuildHasher>;

// Salt used for encrypting passwords within database.
pub const SALT: &[u8] = b"ThisIsMySalt";
