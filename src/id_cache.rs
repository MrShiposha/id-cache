pub type Id = usize;

#[derive(Debug)]
pub struct IdCache {
    top_id: usize,
    pub(crate) free_ids: Vec<Id>,
}

impl IdCache {
    pub fn new() -> Self {
        Self {
            top_id: 0,
            free_ids: Default::default(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            top_id: capacity.into(),
            free_ids: (0..capacity).rev().collect(),
        }
    }

    pub fn acquire_id(&mut self) -> Id {
        match self.try_acquire_id() {
            Some(id) => id,
            None => {
                let old_top_id = self.top_id;
                self.top_id += 1;

                old_top_id
            },
        }
    }

    pub fn try_acquire_id(&mut self) -> Option<Id> {
        self.free_ids.pop()
    }

    /// # Safety
    /// `id` must not be already released.
    ///
    /// # Panics
    /// When `id >= self.top_id`.
    pub unsafe fn release_id(&mut self, id: Id) {
        assert!(id < self.top_id);

        self.free_ids.push(id);
    }

    /// # Safety
    /// `ids` must contain only unique elements.
    /// If `ids` contain duplicates - behavior is undefined.
    ///
    /// # Panics
    /// When some `id` from the `ids` is >= `self.top_id`.
    pub unsafe fn release_ids<I: IntoIterator<Item = Id>>(&mut self, ids: I) {
        let ids = ids.into_iter();

        let top_id = self.top_id;
        self.free_ids.extend(ids.inspect(|&id| {
            assert!(id < top_id);
        }));
    }

    pub fn reset(&mut self) {
        self.top_id = 0;
        self.free_ids.clear();
    }

    pub fn free_ids_num(&self) -> usize {
        self.free_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::IdCache,
        std::{collections::HashSet, iter::FromIterator},
    };

    #[test]
    fn test_id_cache() {
        let mut cache = IdCache::new();
        assert_eq!(cache.top_id, 0);
        assert!(cache.free_ids.is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 0);
        assert_eq!(cache.top_id, 1);
        assert!(cache.free_ids.is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 1);
        assert_eq!(cache.top_id, 2);
        assert!(cache.free_ids.is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 2);
        assert_eq!(cache.top_id, 3);
        assert!(cache.free_ids.is_empty());

        unsafe { cache.release_id(2) }
        assert_eq!(cache.top_id, 3);
        assert_eq!(cache.free_ids, vec![2]);

        unsafe { cache.release_id(1) }
        assert_eq!(cache.top_id, 3);
        assert_eq!(cache.free_ids, vec![2, 1]);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 1);
        assert_eq!(cache.top_id, 3);
        assert_eq!(cache.free_ids, vec![2]);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 2);
        assert_eq!(cache.top_id, 3);
        assert!(cache.free_ids.is_empty());

        let new_id = cache.acquire_id();
        assert_eq!(new_id, 3);
        assert_eq!(cache.top_id, 4);
        assert!(cache.free_ids.is_empty());
    }

    #[test]
    fn test_id_cache_remove_ids() {
        let mut cache = IdCache::new();

        let mut ids = vec![];

        ids.push(cache.acquire_id());

        ids.push(cache.acquire_id());

        ids.push(cache.acquire_id());

        ids.push(cache.acquire_id());

        ids.push(cache.acquire_id());

        unsafe { cache.release_ids(ids.clone()) }

        let mut new_ids = vec![];
        for _ in 0..ids.len() {
            new_ids.push(cache.acquire_id())
        }

        let ids: HashSet<_> = HashSet::from_iter(ids);
        let new_ids = HashSet::from_iter(new_ids);

        assert_eq!(new_ids, ids);
    }

    #[test]
    fn test_id_cache_with_capacity() {
        let capacity = 10;
        let mut cache = IdCache::with_capacity(capacity);
        assert_eq!(cache.top_id, capacity);
        assert_eq!(
            cache.free_ids,
            vec![9, 8, 7, 6, 5, 4, 3, 2, 1, 0]
        );
        assert_eq!(cache.free_ids_num(), capacity);

        for i in 1..=capacity {
            cache.acquire_id();
            assert_eq!(
                cache.free_ids,
                (i..capacity).rev().collect::<Vec<_>>()
            );
            assert_eq!(cache.free_ids_num(), capacity - i);
        }

        assert_eq!(cache.top_id, capacity);
        assert_eq!(cache.free_ids, vec![]);
        assert_eq!(cache.free_ids_num(), 0);

        let new_id = cache.acquire_id();
        assert_eq!(new_id, capacity);
        assert_eq!(cache.top_id, capacity + 1);
        assert_eq!(cache.free_ids_num(), 0);

        unsafe { cache.release_id(9) }
        assert_eq!(cache.top_id, capacity + 1);
        assert_eq!(cache.free_ids, vec![9]);
        assert_eq!(cache.free_ids_num(), 1);
    }

    #[test]
    fn test_try_acquire_id() {
        let mut cache = IdCache::new();

        assert!(cache.try_acquire_id().is_none());

        let src_id = cache.acquire_id();
        unsafe { cache.release_id(src_id) }
        let freed_id = cache.try_acquire_id();
        assert!(freed_id.is_some());
        assert_eq!(freed_id.unwrap(), src_id);
    }
}
