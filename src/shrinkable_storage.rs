use {
    super::Id,
    std::iter::Extend
};

#[derive(Debug, Clone)]
pub struct ShrinkableStorage<T> {
    data: Vec<T>,
    free_ids: Vec<Id>
}

impl<T> ShrinkableStorage<T> {
    pub fn new() -> Self {
        Self {
            data: vec![],
            free_ids: vec![],
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            free_ids: vec![],
        }
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
    /// * If `id` was already released
    pub fn free_id(&mut self, id: Id) {
        debug_assert!(
            self.free_ids.iter().find(|&&free_id| free_id == id).is_none(),
            "id double free"
        );
        debug_assert!(id < self.data.len());

        self.free_ids.push(id);
    }

    /// # Safety
    /// `ids` must contain only unique elements.
    /// If `ids` contain duplicates - behavior is undefined.
    ///
    /// # Panics
    /// When some `id` from the `ids` is greater than the last allocated id.
    pub unsafe fn free_ids(&mut self, ids: impl IntoIterator<Item=Id>) {
        let ids = ids.into_iter();

        let last_id = self.data.len();
        self.free_ids.extend(ids.inspect(|&id| {
            debug_assert!(id < last_id);
        }));
    }

    pub fn iter(&self) -> impl Iterator<Item=(Id, &T)> {
        self.data.iter().enumerate()
    }
}

impl<T: Clone> ShrinkableStorage<T> {
    /// Returns new storage without freed elements.
    /// # Note
    /// Ids in the new storage will change.
    pub fn shrink(&self) -> Self {
        let mut storage = self.clone();

        storage.free_ids.sort_unstable();
        while let Some(id) = storage.free_ids.pop() {
            storage.data.swap_remove(id);
        }

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
        std::collections::HashSet
    };

    #[test]
    fn test_shrinkable_storage() {
        let src_data: HashSet<_> = [1, 2, 3, 4, 5, 6, 7, 8, 9].iter().collect();
        let new_data: HashSet<_> = [1, 2,    4,       7, 8   ].iter().collect();

        let mut storage = ShrinkableStorage::new();
        storage.extend(src_data.clone());

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

        unsafe {
            storage.free_ids(remove_ids);
        }

        assert_eq!(storage.free_ids.len(), 3);

        let remove_id = storage.iter()
            .find_map(|(id, &&obj)| if obj == 9 {
                Some(id)
            } else {
                None
            })
            .unwrap();

        storage.free_id(remove_id);

        assert_eq!(storage.free_ids.len(), 4);

        let new_storage = storage.shrink();
        let stored_data: HashSet<_> = new_storage.iter()
            .map(|(_id, obj)| obj.clone())
            .collect();

        assert_eq!(stored_data, new_data);
        assert_eq!(storage.free_ids.len(), 4);
        assert!(new_storage.free_ids.is_empty());
    }
}