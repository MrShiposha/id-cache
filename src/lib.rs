mod cache_storage;
mod id_cache;
mod shrinkable_storage;

pub use crate::{
    id_cache::*,
    cache_storage::CacheStorage,
    shrinkable_storage::ShrinkableStorage,
};

pub type Id = usize;
