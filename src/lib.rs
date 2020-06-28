use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering}
};

pub type Id = usize;

#[derive(Debug)]
pub struct IdCache {
    top_id: AtomicUsize,
    free_ids: Mutex<Vec<Id>>
}

impl IdCache {
    pub fn new() -> Self {
        Self {
            top_id: AtomicUsize::new(0),
            free_ids: Mutex::default()
        }
    }

    pub fn acquire_id(&self) -> Id {
        match self.free_ids.lock().unwrap().pop() {
            Some(id) => id,
            None => self.top_id.fetch_add(1, Ordering::AcqRel)
        }
    }
    
    pub fn release_id(&self, id: Id) {
        self.free_ids.lock().unwrap().push(id);
    }

    pub fn reset(&self) {
        self.top_id.store(0, Ordering::Release);
        self.free_ids.lock().unwrap().clear();
    }
}


#[cfg(test)]
mod tests {
    use {
        std::sync::atomic::Ordering,
        crate::IdCache
    };

    #[test]
    fn test_id_cache() {

        let cache = IdCache::new();
        assert_eq!(cache.top_id.load(Ordering::Acquire), 0);
        assert!(cache.free_ids.lock().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 0);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 1);
        assert!(cache.free_ids.lock().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 1);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 2);
        assert!(cache.free_ids.lock().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 2);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert!(cache.free_ids.lock().unwrap().is_empty());

        cache.release_id(2);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert_eq!(*cache.free_ids.lock().unwrap(), vec![2]);

        cache.release_id(1);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert_eq!(*cache.free_ids.lock().unwrap(), vec![2, 1]);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 1);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert_eq!(*cache.free_ids.lock().unwrap(), vec![2]);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 2);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 3);
        assert!(cache.free_ids.lock().unwrap().is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 3);
        assert_eq!(cache.top_id.load(Ordering::Acquire), 4);
        assert!(cache.free_ids.lock().unwrap().is_empty());
    }
}