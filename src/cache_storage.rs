use {
    super::{
        id_cache::*, Id
    },
    std::iter::Extend
};

pub struct CacheStorage<T> {
    data: Vec<T>,
    id_cache: IdCache,
}

impl<T> CacheStorage<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::default(),
            id_cache: IdCache::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            id_cache: IdCache::with_capacity(capacity),
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

    /// # Panics
    /// [DEBUG CFG]
    /// * If `id` is greater than the last allocated id.
    /// * If `id` was already released
    pub fn remove(&mut self, id: Id) {
        self.id_cache.release_id(id);
    }

    /// # Safety
    /// `ids` must contain only unique elements.
    /// If `ids` contain duplicates - behavior is undefined.
    ///
    /// # Panics
    /// When some `id` from the `ids` is greater than the last allocated id.
    pub unsafe fn remove_chunk<I: IntoIterator<Item = Id>>(&mut self, ids: I) {
        self.id_cache.release_ids(ids);
    }

    /// # Safety
    /// It is safe to call this function,
    /// but several removed elements may still stay in the collection,
    /// so the corresponding ids were released.
    pub unsafe fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.data.iter().enumerate()
    }

    /// # Safety
    /// It is safe to call this function,
    /// but several removed elements may still stay in the vector.
    pub unsafe fn into_vec(self) -> Vec<T> {
        self.data
    }
}

impl<T> Extend<T> for CacheStorage<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.insert(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::CacheStorage;

    #[test]
    fn test_cache_storage() {
        macro_rules! collect_data {
            ($storage:expr) => {
                unsafe {
                    $storage
                        .iter()
                        .map(|(id, data)| (id, data.clone()))
                        .collect::<Vec<_>>()
                }
            };
        }

        let mut storage: CacheStorage<usize> = CacheStorage::new();

        assert_eq!(storage.data.len(), 0);
        assert_eq!(collect_data![storage], vec![]);

        let first_id = storage.insert(42);
        assert_eq!(first_id, 0);
        assert_eq!(storage.data.len(), 1);
        assert_eq!(*storage.get(first_id), 42);
        *storage.get_mut(first_id) *= 2;
        assert_eq!(*storage.get(first_id), 42 * 2);
        assert_eq!(collect_data![storage], vec![(first_id, 42 * 2)]);

        let second_id = storage.insert(111);
        assert_eq!(second_id, 1);
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.get(second_id), 111);
        *storage.get_mut(second_id) *= 2;
        assert_eq!(*storage.get(second_id), 111 * 2);
        assert_eq!(
            collect_data![storage],
            vec![(first_id, 42 * 2), (second_id, 111 * 2)]
        );

        storage.remove(first_id);
        assert_eq!(storage.data.len(), 2);
        assert_eq!(
            collect_data![storage],
            vec![(first_id, 42 * 2), (second_id, 111 * 2)]
        );

        let first_id = storage.insert(10);
        assert_eq!(first_id, 0);
        assert_eq!(storage.data.len(), 2);
        assert_eq!(*storage.get(first_id), 10);
        *storage.get_mut(first_id) *= 2;
        assert_eq!(*storage.get(first_id), 10 * 2);
        assert_eq!(
            collect_data![storage],
            vec![(first_id, 10 * 2), (second_id, 111 * 2)]
        );

        let storage = CacheStorage::<i32>::with_capacity(10);
        assert_eq!(storage.data.capacity(), 10);
        assert_eq!(storage.data.len(), 0);
        assert_eq!(collect_data![storage], vec![]);
    }

    #[test]
    fn test_cache_storage_try_insert() {
        let mut storage = CacheStorage::with_capacity(3);
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
    fn test_cache_storage_into_vec() {
        let mut storage = CacheStorage::new();
        let range = 0..5;
        let expected = range.clone().collect::<Vec<_>>();

        for i in range {
            storage.insert(i);
        }

        let stored = unsafe { storage.into_vec() };

        assert_eq!(stored, expected);
    }

    #[test]
    fn test_cache_storage_extend() {
        let mut storage = CacheStorage::with_capacity(5);
        storage.extend(vec![1, 2, 3]);

        assert_eq!(storage.data, vec![1, 2, 3]);
        assert_eq!(storage.id_cache.free_ids, vec![4, 3]);

        storage.extend(vec![4, 5, 6]);
        assert_eq!(storage.data, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(storage.id_cache.free_ids, vec![]);
    }
}
