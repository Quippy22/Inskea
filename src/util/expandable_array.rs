use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

/// A compact growable array with an explicit starting capacity.
///
/// Growing keeps values in the same logical order and at the same indices.
/// A successful `Vec` reallocation may move the backing memory, so callers
/// must not retain pointers into the array across growth.
#[derive(Clone, Debug)]
pub struct ExpandableArray<T> {
    values: Vec<T>,
    pub initial_size: usize,
}

impl<T: PartialEq> PartialEq for ExpandableArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl<T: Eq> Eq for ExpandableArray<T> {}

impl<T> ExpandableArray<T> {
    /// Create an empty array and reserve the requested initial capacity when
    /// the allocator can provide it.
    pub fn new(initial_size: usize) -> Self {
        let mut values = Vec::new();
        let _ = values.try_reserve_exact(initial_size);
        Self {
            values,
            initial_size,
        }
    }

    /// Build an array from values, reserving at least `initial_size` slots.
    pub fn from_vec(initial_size: usize, values: Vec<T>) -> Self {
        let mut array = Self {
            values,
            initial_size,
        };
        let _ = array
            .values
            .try_reserve_exact(initial_size.saturating_sub(array.values.len()));
        array
    }

    /// Build an array from an iterator with an explicit initial capacity.
    pub fn from_iter_with_capacity(
        initial_size: usize,
        values: impl IntoIterator<Item = T>,
    ) -> Self {
        let mut array = Self::new(initial_size);
        array.extend(values);
        array
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn capacity(&self) -> usize {
        self.values.capacity()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.values
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.values
    }

    /// Expand storage to at least `new_size` slots, preserving every value
    /// and its index. If the requested allocation cannot be made, the
    /// original array is returned intact.
    #[allow(dead_code)]
    pub fn expand(mut self, new_size: usize) -> Self {
        if new_size > self.capacity() {
            let additional = new_size.saturating_sub(self.values.len());
            let _ = self.values.try_reserve_exact(additional);
        }
        self
    }

    /// Append a value. Returns `false` and leaves the array intact if growth
    /// cannot be reserved safely.
    pub fn push(&mut self, value: T) -> bool {
        if self.values.len() == self.values.capacity() {
            let next_size = self
                .values
                .capacity()
                .saturating_mul(2)
                .max(self.initial_size)
                .max(self.values.len().saturating_add(1));
            if self
                .values
                .try_reserve_exact(next_size.saturating_sub(self.values.len()))
                .is_err()
            {
                return false;
            }
        }
        self.values.push(value);
        true
    }

    /// Insert at an existing index, or at the end. Invalid indices and
    /// allocation failures leave the array unchanged.
    pub fn insert(&mut self, index: usize, value: T) -> bool {
        if index > self.values.len() {
            return false;
        }
        if self.values.len() == self.values.capacity() {
            let next_size = self
                .values
                .capacity()
                .saturating_mul(2)
                .max(self.initial_size)
                .max(self.values.len().saturating_add(1));
            if self
                .values
                .try_reserve_exact(next_size.saturating_sub(self.values.len()))
                .is_err()
            {
                return false;
            }
        }
        self.values.insert(index, value);
        true
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        (index < self.values.len()).then(|| self.values.remove(index))
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        for value in values {
            if !self.push(value) {
                break;
            }
        }
    }
}

impl<T> Deref for ExpandableArray<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<T> DerefMut for ExpandableArray<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl<T> FromIterator<T> for ExpandableArray<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let values: Vec<T> = iter.into_iter().collect();
        let initial_size = values.capacity();
        Self {
            values,
            initial_size,
        }
    }
}

impl<T: Serialize> Serialize for ExpandableArray<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.values.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ExpandableArray<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = Vec::<T>::deserialize(deserializer)?;
        let initial_size = values.len();
        Ok(Self {
            values,
            initial_size,
        })
    }
}
