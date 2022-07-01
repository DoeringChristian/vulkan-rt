use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;

use crate::dense_arena::DefaultKey;
use crate::dense_arena::DenseArena;
use crate::dense_arena::DenseStatusArena;

pub struct RenderWorldKey<T> {
    key: DefaultKey,
    _ty: PhantomData<T>,
}

impl<T> Copy for RenderWorldKey<T> {}
impl<T> Clone for RenderWorldKey<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            _ty: PhantomData,
        }
    }
}

pub enum RenderWorldStatus {
    Recreated,
    Changed,
    Unchaged,
}
impl Default for RenderWorldStatus {
    fn default() -> Self {
        Self::Recreated
    }
}

type RenderWorldArena<T> = (
    DenseStatusArena<DefaultKey, T, RenderWorldStatus>,
    RenderWorldStatus,
);

#[derive(Default)]
pub struct RenderWorld {
    map: HashMap<TypeId, Box<dyn Any>>,
}
impl RenderWorld {
    //type Arena<T> = DenseArena<DefaultKey, T>;
    pub fn insert<T: 'static>(&mut self, val: T) -> RenderWorldKey<T> {
        let ty_id = TypeId::of::<T>();
        RenderWorldKey {
            key: self
                .map
                .entry(ty_id)
                .or_insert(Box::new((
                    DenseStatusArena::<DefaultKey, T, RenderWorldStatus>::default(),
                    RenderWorldStatus::Recreated,
                )))
                .downcast_mut::<RenderWorldArena<T>>()
                .unwrap()
                .0
                .insert(val),
            _ty: PhantomData,
        }
    }
    pub fn remove<T: 'static>(&mut self, key: RenderWorldKey<T>) -> Option<T> {
        self.map
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<RenderWorldArena<T>>()?
            .0
            .remove(&key.key)
    }
    pub fn get<T: 'static>(&self, key: RenderWorldKey<T>) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())?
            .downcast_ref::<RenderWorldArena<T>>()?
            .0
            .get(key.key)
    }
    pub fn get_mut<T: 'static>(&mut self, key: RenderWorldKey<T>) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<RenderWorldArena<T>>()?
            .0
            .get_mut(key.key)
    }
    pub fn iter<T: 'static>(&mut self) -> Option<impl Iterator<Item = (RenderWorldKey<T>, &T)>> {
        Some(
            self.map
                .get(&TypeId::of::<T>())?
                .downcast_ref::<RenderWorldArena<T>>()?
                .0
                .iter()
                .map(|(k, v)| {
                    (
                        RenderWorldKey {
                            key: *k,
                            _ty: PhantomData,
                        },
                        v,
                    )
                }),
        )
    }
    pub fn iter_mut<T: 'static>(
        &mut self,
    ) -> Option<impl Iterator<Item = (RenderWorldKey<T>, &mut T)>> {
        Some(
            self.map
                .get_mut(&TypeId::of::<T>())?
                .downcast_mut::<RenderWorldArena<T>>()?
                .0
                .iter_mut()
                .map(|(k, v)| {
                    (
                        RenderWorldKey {
                            key: *k,
                            _ty: PhantomData,
                        },
                        v,
                    )
                }),
        )
    }
    pub fn values<T: 'static>(&self) -> Option<impl Iterator<Item = &T>> {
        Some(
            self.map
                .get(&TypeId::of::<T>())?
                .downcast_ref::<RenderWorldArena<T>>()?
                .0
                .values(),
        )
    }
    pub fn values_mut<T: 'static>(&mut self) -> Option<impl Iterator<Item = &mut T>> {
        Some(
            self.map
                .get_mut(&TypeId::of::<T>())?
                .downcast_mut::<RenderWorldArena<T>>()?
                .0
                .values_mut(),
        )
    }
    pub fn get_dense_index<T: 'static>(&self, key: RenderWorldKey<T>) -> Option<usize> {
        self.map
            .get(&TypeId::of::<T>())?
            .downcast_ref::<RenderWorldArena<T>>()?
            .0
            .get_dense_index(key.key)
    }
    pub fn dense_index<T: 'static>(&self, key: RenderWorldKey<T>) -> usize {
        self.get_dense_index(key).unwrap()
    }
    pub fn get_key<T: 'static>(&self, index: usize) -> Option<RenderWorldKey<T>> {
        Some(RenderWorldKey {
            key: self
                .map
                .get(&TypeId::of::<T>())?
                .downcast_ref::<RenderWorldArena<T>>()?
                .0
                .get_key(index)?,
            _ty: PhantomData,
        })
    }
    pub fn key<T: 'static>(&self, index: usize) -> RenderWorldKey<T> {
        self.get_key(index).unwrap()
    }
}
impl<T: 'static> std::ops::Index<RenderWorldKey<T>> for RenderWorld {
    type Output = T;

    fn index(&self, index: RenderWorldKey<T>) -> &Self::Output {
        self.get(index).unwrap()
    }
}
impl<T: 'static> std::ops::IndexMut<RenderWorldKey<T>> for RenderWorld {
    fn index_mut(&mut self, index: RenderWorldKey<T>) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::RenderWorld;

    #[test]
    fn test0() {
        let mut world = RenderWorld::default();
        let i0 = world.insert(0);
        let i1 = world.insert(1);
        let i2 = world.insert(2);

        assert_eq!(world.get(i0), Some(&0));
    }
}
