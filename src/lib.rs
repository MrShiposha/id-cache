use std::sync::{
    RwLock,
    atomic::{AtomicUsize, Ordering}
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

    pub fn acquire_id(&self) -> Id {
        match self.free_ids.write().unwrap().pop() {
            Some(id) => id,
            None => self.top_id.fetch_add(1, Ordering::AcqRel)
        }
    }

    pub fn release_id(&self, id: Id) {
        self.free_ids.write().unwrap().push(id);
    }

    pub fn reset(&self) {
        self.top_id.store(0, Ordering::Release);
        self.free_ids.write().unwrap().clear();
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
            id_cache: IdCache::new()
        }
    }

    pub fn insert(&mut self, new_data: T) -> Id {
        let id = self.id_cache.acquire_id();
        self.insert_with_id(id, new_data);

        id
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

    pub fn reset_ids(&self) {
        self.id_cache.reset();
    }

    pub fn is_empty(&self) -> bool {
        self.id_cache.top_id.load(Ordering::Acquire) == 0
    }

    pub fn data(&self) -> &Vec<T> {
        &self.data
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
    fn test_storage() {
        let mut storage = Storage::new();
        assert!(storage.is_empty());
        assert_eq!(storage.data.len(), 0);
        assert_eq!(*storage.data(), vec![]);


        let id = storage.insert(42);
        assert_eq!(id, 0);
        assert!(!storage.is_empty());
        assert_eq!(storage.data.len(), 1);
        assert_eq!(*storage.get(id), 42);
        *storage.get_mut(id) *= 2;
        assert_eq!(*storage.get(id), 42 * 2);
        assert_eq!(*storage.data(), vec![42 * 2]);

        let first_id = id;

        let id = storage.insert(111);
        assert_eq!(id, 1);
        assert!(!storage.is_empty());
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.get(id), 111);
        *storage.get_mut(id) *= 2;
        assert_eq!(*storage.get(id), 111 * 2);
        assert_eq!(*storage.data(), vec![42 * 2, 111 * 2]);

        storage.remove(first_id);
        assert!(!storage.is_empty());
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.data(), vec![42 * 2, 111 * 2]);

        let id = storage.insert(10);
        assert_eq!(id, 0);
        assert!(!storage.is_empty());
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.get(id), 10);
        *storage.get_mut(id) *= 2;
        assert_eq!(*storage.get(id), 10 * 2);
        assert_eq!(*storage.data(), vec![10 * 2, 111 * 2]);

        storage.reset_ids();
        assert!(storage.is_empty());
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.data(), vec![10 * 2, 111 * 2]);

        let id = storage.insert(42);
        assert_eq!(id, 0);
        assert!(!storage.is_empty());
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.get(id), 42);
        *storage.get_mut(id) *= 2;
        assert_eq!(*storage.get(id), 42 * 2);
        assert_eq!(*storage.data(), vec![42 * 2, 111 * 2]);

        let storage = Storage::<i32>::with_capacity(10);
        assert!(storage.is_empty());
        assert_eq!(storage.data.capacity(), 10);
        assert_eq!(storage.data.len(), 0);
        assert_eq!(*storage.data(), vec![]);
    }
}