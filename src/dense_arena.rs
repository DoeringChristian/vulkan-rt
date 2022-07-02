use std::{
    hash::Hash,
    marker::PhantomData,
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

#[derive(Debug)]
pub struct DenseArena<K: Key, V, S = ()> {
    values: Vec<V>,
    keys: Vec<K>,
    slots: Vec<(Slot, S)>,
    free: usize,
}
impl<K: Key, V, S> Default for DenseArena<K, V, S> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            keys: Vec::new(),
            slots: Vec::new(),
            free: 0,
        }
    }
}

impl<K: Key, V, S: Default> DenseArena<K, V, S> {
    #[must_use]
    pub fn insert(&mut self, value: V) -> K {
        let key = match self.slots.get_mut(self.free) {
            Some((slot, _)) if slot.version % 2 == 0 => {
                slot.version += 1;
                let key = KeyData {
                    idx: self.free,
                    version: slot.version,
                };
                self.free = slot.idx_or_free;
                slot.idx_or_free = self.values.len();
                key
            }
            _ => {
                self.slots.push((
                    Slot {
                        version: 1,
                        idx_or_free: self.values.len(),
                    },
                    S::default(),
                ));
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
        if self.slots[key.idx].0.version != key.version {
            return None;
        }
        let idx = self.slots[key.idx].0.idx_or_free;
        self.slots[key.idx].0.version += 1;
        self.slots[key.idx].0.idx_or_free = self.free;
        self.free = key.idx;
        let _ = self.keys.swap_remove(idx);
        let value = self.values.swap_remove(idx);
        if idx < self.values.len() {
            // update slot if swap_remove swaped
            self.slots[self.keys[idx].key_data().idx].0.idx_or_free = idx;
        }
        Some(value)
    }
    pub fn get(&self, key: K) -> Option<&V> {
        let key = key.key_data();
        let slot = self.slots.get(key.idx)?;
        if slot.0.version == key.version && key.version % 2 != 0 {
            Some(self.values.get(slot.0.idx_or_free)?)
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        let key = key.key_data();
        let slot = self.slots.get(key.idx)?;
        if slot.0.version == key.version && key.version % 2 != 0 {
            Some(self.values.get_mut(slot.0.idx_or_free)?)
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
        if slot.0.version == key.version && key.version % 2 != 0 {
            Some(slot.0.idx_or_free)
        } else {
            None
        }
    }
    pub fn status(&self, key: K) -> &S {
        self.get_status(key).unwrap()
    }
    pub fn status_mut(&mut self, key: K) -> &mut S {
        self.get_status_mut(key).unwrap()
    }
    pub fn get_status(&self, key: K) -> Option<&S> {
        let key = key.key_data();
        let slot = self.slots.get(key.idx)?;
        if slot.0.version == key.version && key.version % 2 != 0 {
            Some(&self.slots.get(key.idx)?.1)
        } else {
            None
        }
    }
    pub fn get_status_mut(&mut self, key: K) -> Option<&mut S> {
        let key = key.key_data();
        let slot = self.slots.get(key.idx)?;
        if slot.0.version == key.version && key.version % 2 != 0 {
            Some(&mut self.slots.get_mut(key.idx)?.1)
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
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
impl<K: Key, V, S: Default> Index<K> for DenseArena<K, V, S> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        self.get(index).unwrap()
    }
}
impl<K: Key, V, S: Default> IndexMut<K> for DenseArena<K, V, S> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

//pub type DenseArena<K, V> = DenseStatusArena<K, V, ()>;

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

#[cfg(test)]
mod test {
    use crate::dense_arena::DefaultKey;

    use super::DenseArena;

    #[test]
    fn test() {
        let mut dense_arena = DenseArena::<DefaultKey, i32>::default();
        let i0 = dense_arena.insert(0);
        let i1 = dense_arena.insert(1);
        //assert_eq!(dense_arena.get(i0), Some(&0));
        println!("{:#?}", dense_arena);
        dense_arena.remove(&i1);
        println!("{:#?}", dense_arena);
        assert_eq!(dense_arena.get(i1), None);
    }
}
