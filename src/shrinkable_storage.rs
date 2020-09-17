use {
    super::Id,
    std::{
        iter::Extend,
        collections::BTreeSet,
    }
};

#[derive(Debug, Clone)]
pub struct ShrinkableStorage<T> {
    data: Vec<T>,
    free_ids: BTreeSet<Id>
}

impl<T> ShrinkableStorage<T> {
    pub fn new() -> Self {
        Self {
            data: vec![],
            free_ids: BTreeSet::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            free_ids: BTreeSet::new(),
        }
    }

    pub fn volume(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn insert(&mut self, obj: T) -> Id {
        let id = self.data.len();
        self.data.push(obj);

        id
    }

    pub fn get(&self, id: Id) -> &T {
        &self.data[id]
    }

    pub fn get_mut(&mut self, id: Id) -> &mut T {
        &mut self.data[id]
    }

    /// # Panics
    /// [DEBUG CFG]
    /// * If `id >= self.data.len()`
    pub fn free_id(&mut self, id: Id) {
        debug_assert!(id < self.data.len());

        self.free_ids.insert(id);
    }

    /// # Panics
    /// When some `id` from the `ids` is greater than the last allocated id.
    pub fn free_ids(&mut self, ids: impl IntoIterator<Item=Id>) {
        let ids = ids.into_iter();

        let last_id = self.data.len();
        self.free_ids.extend(ids.inspect(|&id| {
            debug_assert!(id < last_id);
        }));
    }

    pub fn is_id_free(&mut self, id: &Id) -> bool {
        self.free_ids.contains(id)
    }

    /// # Safety
    /// This function will not free the ids.
    pub unsafe fn retain<P>(&mut self, predicate: P)
    where
        P: FnMut(&T) -> bool
    {
        self.data.retain(predicate);
    }

    pub fn restore_freed(&mut self) {
        self.free_ids.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item=(Id, &T)> {
        self.data.iter().enumerate()
    }

    pub fn iter_ids(&self) -> impl Iterator<Item=Id> {
        0..self.data.len()
    }
}

impl<T: Clone> ShrinkableStorage<T> {
    /// Returns new storage without freed elements.
    /// # Note
    /// Ids in the new storage will change.
    pub fn shrink(&self) -> Self {
        let mut storage = self.clone();

        let mut iter = storage.free_ids.iter();
        while let Some(&id) = iter.next_back() {
            storage.data.swap_remove(id);
        }

        storage.free_ids.clear();

        storage
    }
}

impl<T> Extend<T> for ShrinkableStorage<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.data.extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::ShrinkableStorage,
        std::{
            collections::HashSet,
            iter::once,
        }
    };

    #[test]
    fn test_shrinkable_storage() {
        let src_data: HashSet<_> = [1, 2, 3, 4, 5, 6, 7, 8, 9].iter().collect();
        let new_data: HashSet<_> = [1, 2,    4,       7, 8   ].iter().collect();

        let mut storage = ShrinkableStorage::new();
        assert!(storage.is_empty());
        assert_eq!(storage.volume(), 0);

        storage.extend(src_data.clone());
        assert!(!storage.is_empty());
        assert_eq!(storage.volume(), src_data.len());

        let stored_data: HashSet<_> = storage.iter()
            .map(|(_id, obj)| obj.clone())
            .collect();

        assert_eq!(stored_data, src_data);

        let remove_ids: Vec<_> = storage.iter()
            .filter_map(|(id, obj)| match obj {
                3 | 5 | 6 => Some(id),
                _ => None
            })
            .collect();

        storage.free_ids(remove_ids.clone());

        assert_eq!(storage.free_ids.len(), 3);

        let remove_id = storage.iter()
            .find_map(|(id, &&obj)| if obj == 9 {
                Some(id)
            } else {
                None
            })
            .unwrap();

        storage.free_id(remove_id);

        for id in remove_ids.iter().chain(once(&remove_id)) {
            assert!(storage.is_id_free(id));
        }

        assert_eq!(storage.free_ids.len(), 4);
        assert!(!storage.is_empty());
        assert_eq!(storage.volume(), src_data.len());

        let new_storage = storage.shrink();
        let stored_data: HashSet<_> = new_storage.iter()
            .map(|(_id, obj)| obj.clone())
            .collect();

        assert_eq!(stored_data, new_data);
        assert_eq!(storage.free_ids.len(), 4);
        assert!(!storage.is_empty());
        assert_eq!(storage.volume(), src_data.len());
        assert!(new_storage.free_ids.is_empty());
        assert!(!new_storage.is_empty());
        assert_eq!(new_storage.volume(), new_data.len());
    }
}