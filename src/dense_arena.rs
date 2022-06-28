use std::{
    hash::Hash,
    ops::{Index, IndexMut},
};

pub trait Key:
    From<KeyData>
    + Copy
    + Clone
    + Eq
    + PartialEq
    + Ord
    + PartialOrd
    + core::hash::Hash
    + core::fmt::Debug
{
    fn key_data(&self) -> KeyData;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct KeyData {
    idx: usize,
    // even = vacant, odd = occupied
    version: usize,
}

#[derive(Debug, Clone)]
pub struct Slot {
    idx_or_free: usize,
    version: usize,
}

///
/// Own implementation of DenseSlotMap with acces to values as slice.
///
#[derive(Debug, Clone)]
pub struct DenseArena<K: Key, V> {
    values: Vec<V>,
    keys: Vec<K>,
    slots: Vec<Slot>,
    free: usize,
}

impl<K: Key, V> Default for DenseArena<K, V> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            keys: Vec::new(),
            slots: Vec::new(),
            free: 0,
        }
    }
}

impl<K: Key, V> DenseArena<K, V> {
    #[must_use]
    pub fn insert(&mut self, value: V) -> K {
        let key = match self.slots.get_mut(self.free) {
            Some(slot) if slot.version % 2 == 0 => {
                slot.version += 1;
                let key = KeyData {
                    idx: self.free,
                    version: slot.version + 1,
                };
                self.free = slot.idx_or_free;
                slot.idx_or_free = self.values.len();
                key
            }
            _ => {
                self.slots.push(Slot {
                    version: 1,
                    idx_or_free: self.values.len(),
                });
                KeyData {
                    version: 1,
                    idx: self.slots.len() - 1,
                }
            }
        };
        self.values.push(value);
        let key = K::from(key);
        self.keys.push(key);
        key
    }
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let key = key.key_data();
        if self.slots[key.idx].version != key.version {
            return None;
        }
        let idx = self.slots[key.idx].idx_or_free;
        self.slots[key.idx].version += 1;
        self.slots[key.idx].idx_or_free = self.free;
        self.free = key.idx;
        let _ = self.keys.swap_remove(idx);
        let value = self.values.swap_remove(idx);
        self.slots[self.keys[idx].key_data().idx].idx_or_free = idx;
        Some(value)
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let key = key.key_data();
        let slot = self.slots.get(key.idx)?;
        if slot.version == key.version && key.version % 2 != 0 {
            Some(self.values.get(slot.idx_or_free)?)
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let key = key.key_data();
        let slot = self.slots.get(key.idx)?;
        if slot.version == key.version && key.version % 2 != 0 {
            Some(self.values.get_mut(slot.idx_or_free)?)
        } else {
            None
        }
    }
    pub fn as_slices(&self) -> (&[K], &[V]) {
        (&self.keys, &self.values)
    }
    pub fn values_as_slice(&self) -> &[V] {
        &self.values
    }
    pub fn keys_as_slice(&self) -> &[K] {
        &self.keys
    }
    pub fn dense_index(&self, key: K) -> usize {
        self.get_dense_index(key).unwrap()
    }
    pub fn get_dense_index(&self, key: K) -> Option<usize> {
        let key = key.key_data();
        let slot = self.slots.get(key.idx)?;
        if slot.version == key.version && key.version % 2 != 0 {
            Some(slot.idx_or_free)
        } else {
            None
        }
    }
    pub fn key(&self, dense_key: usize) -> K {
        self.get_key(dense_key).unwrap()
    }
    pub fn get_key(&self, dense_key: usize) -> Option<K> {
        self.keys.get(dense_key).map(|k| *k)
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.values.iter()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.values.iter_mut()
    }
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.keys.iter().zip(self.values.iter_mut())
    }
}

impl<K: Key, V> Index<K> for DenseArena<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).unwrap()
    }
}
impl<K: Key, V> IndexMut<K> for DenseArena<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

macro_rules! new_key_type {
    ( $(#[$outer:meta])* $vis:vis struct $name:ident; $($rest:tt)* ) => {
        $(#[$outer])*
        #[derive(Copy, Clone,
                 Eq, PartialEq, Ord, PartialOrd,
                 Hash, Debug)]
        #[repr(transparent)]
        $vis struct $name(KeyData);

        impl From<KeyData> for $name {
            fn from(k: KeyData) -> Self {
                $name(k)
            }
        }

        impl Key for $name {
            fn key_data(&self) -> KeyData {
                self.0
            }
        }
        new_key_type!($($rest)*);
    };

    () => {}
}
new_key_type! {
    pub struct DefaultKey;
}
