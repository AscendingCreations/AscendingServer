mod bases;
mod storage;

pub use bases::*;
pub use storage::*;

//We redefine these here so it is easier to update the hash style later if we need too.
pub type FxBuildHasher = std::hash::BuildHasherDefault<ritehash::FxHasher>;

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, FxBuildHasher>;
pub type IndexSet<T> = indexmap::IndexSet<T, FxBuildHasher>;
pub type HashSet<T> = std::collections::HashSet<T, FxBuildHasher>;
pub type HashMap<K, V> = std::collections::HashMap<K, V, FxBuildHasher>;

pub const SALT: &[u8] = b"ThisIsMySalt";
