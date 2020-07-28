use std::{
    sync::{
        RwLock,
        atomic::{AtomicUsize, Ordering}
    },
    iter::Extend
};

pub type Id = usize;

#[derive(Debug)]
pub struct IdCache {
    top_id: AtomicUsize,
    free_ids: RwLock<Vec<Id>>
}

impl IdCache {
    pub fn new() -> Self {
        Self {
            top_id: AtomicUsize::new(0),
            free_ids: RwLock::default()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            top_id: capacity.into(),
            free_ids: RwLock::new((0..capacity).rev().collect())
        }
    }

    pub fn acquire_id(&self) -> Id {
        match self.try_acquire_id() {
            Some(id) => id,
            None => self.top_id.fetch_add(1, Ordering::AcqRel)
        }
    }

    pub fn try_acquire_id(&self) -> Option<Id> {
        self.free_ids.write().unwrap().pop()
    }

    pub fn release_id(&self, id: Id) {
        assert!(id < self.top_id.load(Ordering::Acquire));

        self.free_ids.write().unwrap().push(id);
    }

    pub fn reset(&self) {
        self.top_id.store(0, Ordering::Release);
        self.free_ids.write().unwrap().clear();
    }

    pub fn free_ids_num(&self) -> usize {
        self.free_ids.read().unwrap().len()
    }
}

pub struct Storage<T> {
    data: Vec<T>,
    id_cache: IdCache
}

impl<T> Storage<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::default(),
            id_cache: IdCache::new()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            id_cache: IdCache::with_capacity(capacity)
        }
    }

    pub fn insert(&mut self, new_data: T) -> Id {
        let id = self.id_cache.acquire_id();
        self.insert_with_id(id, new_data);

        id
    }

    pub fn try_insert(&mut self, new_data: T) -> Option<Id> {
        self.id_cache.try_acquire_id().map(|id| {
            self.insert_with_id(id, new_data);
            id
        })
    }

    pub fn insert_with_id(&mut self, id: Id, new_data: T) {
        let len = self.data.len();
        if id == len {
            self.data.push(new_data);
        } else if id < len {
            self.data[id] = new_data;
        } else {
            panic!("`id` is out of valid range");
        }
    }

    pub fn get(&self, id: Id) -> &T {
        &self.data[id]
    }

    pub fn get_mut(&mut self, id: Id) -> &mut T {
        &mut self.data[id]
    }

    pub fn remove(&self, id: Id) {
        self.id_cache.release_id(id);
    }

    pub fn data(&self) -> &Vec<T> {
        &self.data
    }

    pub unsafe fn into_vec(self) -> Vec<T> {
        self.data
    }
}

impl<T> Extend<T> for Storage<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.insert(item);
        }
    }
}


#[cfg(test)]
mod tests {
    use {
        std::sync::atomic::Ordering,
        crate::{IdCache, Storage}
    };

    #[test]
    fn test_id_cache() {

        let cache = IdCache::new();
        assert_eq!(cache.top_id.load(Ordering::Acquire), 0);
        assert!(cache.free_ids.read().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 0);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 1);
        assert!(cache.free_ids.read().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 1);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 2);
        assert!(cache.free_ids.read().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 2);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert!(cache.free_ids.read().unwrap().is_empty());

        cache.release_id(2);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert_eq!(*cache.free_ids.read().unwrap(), vec![2]);

        cache.release_id(1);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert_eq!(*cache.free_ids.read().unwrap(), vec![2, 1]);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 1);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert_eq!(*cache.free_ids.read().unwrap(), vec![2]);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 2);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert!(cache.free_ids.read().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 3);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 4);
        assert!(cache.free_ids.read().unwrap().is_empty());
    }

    #[test]
    fn test_id_cache_with_capacity() {
        let capacity = 10;
        let cache = IdCache::with_capacity(capacity);
        assert_eq!(cache.top_id.load(Ordering::Acquire), capacity);
        assert_eq!(*cache.free_ids.read().unwrap(), vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
        assert_eq!(cache.free_ids_num(), capacity);

        for i in 1..=capacity {
            cache.acquire_id();
            assert_eq!(*cache.free_ids.read().unwrap(), (i..capacity).rev().collect::<Vec<_>>());
            assert_eq!(cache.free_ids_num(), capacity - i);
        }

        assert_eq!(cache.top_id.load(Ordering::Acquire), capacity);
        assert_eq!(*cache.free_ids.read().unwrap(), vec![]);
        assert_eq!(cache.free_ids_num(), 0);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, capacity);
        assert_eq!(cache.top_id.load(Ordering::Acquire), capacity + 1);
        assert_eq!(cache.free_ids_num(), 0);

        cache.release_id(9);
        assert_eq!(cache.top_id.load(Ordering::Acquire), capacity + 1);
        assert_eq!(*cache.free_ids.read().unwrap(), vec![9]);
        assert_eq!(cache.free_ids_num(), 1);
    }

    #[test]
    fn test_try_acquire_id() {
        let cache = IdCache::new();

        assert!(cache.try_acquire_id().is_none());

        let src_id = cache.acquire_id();
        cache.release_id(src_id);
        let freed_id = cache.try_acquire_id();
        assert!(freed_id.is_some());
        assert_eq!(freed_id.unwrap(), src_id);
    }

    #[test]
    fn test_storage() {
        let mut storage = Storage::new();
        assert_eq!(storage.data.len(), 0);
        assert_eq!(*storage.data(), vec![]);


        let id = storage.insert(42);
        assert_eq!(id, 0);
        assert_eq!(storage.data.len(), 1);
        assert_eq!(*storage.get(id), 42);
        *storage.get_mut(id) *= 2;
        assert_eq!(*storage.get(id), 42 * 2);
        assert_eq!(*storage.data(), vec![42 * 2]);

        let first_id = id;

        let id = storage.insert(111);
        assert_eq!(id, 1);
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.get(id), 111);
        *storage.get_mut(id) *= 2;
        assert_eq!(*storage.get(id), 111 * 2);
        assert_eq!(*storage.data(), vec![42 * 2, 111 * 2]);

        storage.remove(first_id);
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.data(), vec![42 * 2, 111 * 2]);

        let id = storage.insert(10);
        assert_eq!(id, 0);
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.get(id), 10);
        *storage.get_mut(id) *= 2;
        assert_eq!(*storage.get(id), 10 * 2);
        assert_eq!(*storage.data(), vec![10 * 2, 111 * 2]);

        let storage = Storage::<i32>::with_capacity(10);
        assert_eq!(storage.data.capacity(), 10);
        assert_eq!(storage.data.len(), 0);
        assert_eq!(*storage.data(), vec![]);
    }

    #[test]
    fn test_storage_try_insert() {
        let mut storage = Storage::with_capacity(3);
        let id = storage.try_insert(0);
        assert!(id.is_some());
        assert_eq!(*storage.get(id.unwrap()), 0);

        let id = storage.try_insert(1);
        assert!(id.is_some());
        assert_eq!(*storage.get(id.unwrap()), 1);

        let id = storage.try_insert(2);
        assert!(id.is_some());
        assert_eq!(*storage.get(id.unwrap()), 2);
        let last_id = id.unwrap();

        let id = storage.try_insert(3);
        assert!(id.is_none());

        storage.remove(last_id);

        let id = storage.try_insert(3);
        assert!(id.is_some());
        assert_eq!(*storage.get(id.unwrap()), 3);
    }

    #[test]
    fn test_storage_into_vec() {
        let mut storage = Storage::new();
        let range = 0..5;
        let expected = range.clone().collect::<Vec<_>>();

        for i in range {
            storage.insert(i);
        }

        let stored = unsafe {
            storage.into_vec()
        };

        assert_eq!(stored, expected);
    }

    #[test]
    fn test_storage_extend() {
        let mut storage = Storage::with_capacity(5);
        storage.extend(vec![1, 2, 3]);

        assert_eq!(storage.data, vec![1, 2, 3]);
        assert_eq!(*storage.id_cache.free_ids.read().unwrap(), vec![4, 3]);

        storage.extend(vec![4, 5, 6]);
        assert_eq!(storage.data, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(*storage.id_cache.free_ids.read().unwrap(), vec![]);
    }
}