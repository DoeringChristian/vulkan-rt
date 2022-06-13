use crate::accel::{Blas, BlasGeometry, BlasInstance, Tlas};

use super::buffers;
use bevy_ecs::prelude::*;
use bytemuck::cast_slice;
use screen_13::prelude::*;
use slotmap::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use std::{io::BufReader, mem::size_of};
use tobj::*;

//#[derive(Copy)]

pub struct SceneKey<V: 'static> {
    _ty: PhantomData<V>,
    key: DefaultKey,
}

impl<V: 'static> Copy for SceneKey<V> {}

impl<V: 'static> Clone for SceneKey<V> {
    #[inline]
    fn clone(&self) -> Self {
        SceneKey {
            _ty: PhantomData,
            key: self.key,
        }
    }
}

impl<V: 'static> SceneKey<V> {
    pub fn get(self, scene: &Scene) -> Option<&V> {
        scene.get(self)
    }
    pub fn get_mut(self, scene: &mut Scene) -> Option<&mut V> {
        scene.get_mut(self)
    }
}

pub struct Scene {
    pub data: HashMap<TypeId, Box<dyn Any>>,
    pub geometries: SlotMap<DefaultKey, BlasGeometry>,
    pub blases: SlotMap<DefaultKey, Blas>,
    pub instances: SlotMap<DefaultKey, BlasInstance>,
    pub tlas: Option<Tlas>,
}

impl Scene {
    pub fn insert<V: 'static>(&mut self, val: V) -> SceneKey<V> {
        let ty_id = TypeId::of::<V>();
        SceneKey {
            key: self
                .data
                .entry(ty_id)
                .or_insert(Box::new(SlotMap::<DefaultKey, V>::new()))
                .downcast_mut::<SlotMap<DefaultKey, V>>()
                .unwrap()
                .insert(val),
            _ty: PhantomData,
        }
    }
    pub fn get<V: 'static>(&self, key: SceneKey<V>) -> Option<&V> {
        let ty_id = TypeId::of::<V>();
        self.data
            .get(&ty_id)?
            .downcast_ref::<SlotMap<DefaultKey, V>>()?
            .get(key.key)
    }
    pub fn get_mut<V: 'static>(&mut self, key: SceneKey<V>) -> Option<&mut V> {
        let ty_id = TypeId::of::<V>();
        self.data
            .get_mut(&ty_id)?
            .downcast_mut::<SlotMap<DefaultKey, V>>()?
            .get_mut(key.key)
    }
    pub fn iter<V: 'static>(&self) -> Option<impl Iterator<Item = (SceneKey<V>, &V)>> {
        let ty_id = TypeId::of::<V>();
        Some(
            self.data
                .get(&ty_id)?
                .downcast_ref::<SlotMap<DefaultKey, V>>()?
                .iter()
                .map(|(k, v)| {
                    (
                        SceneKey {
                            key: k,
                            _ty: PhantomData,
                        },
                        v,
                    )
                }),
        )
    }
    pub fn iter_mut<V: 'static>(&mut self) -> Option<impl Iterator<Item = (SceneKey<V>, &mut V)>> {
        let ty_id = TypeId::of::<V>();
        Some(
            self.data
                .get_mut(&ty_id)?
                .downcast_mut::<SlotMap<DefaultKey, V>>()?
                .iter_mut()
                .map(|(k, v)| {
                    (
                        SceneKey {
                            key: k,
                            _ty: PhantomData,
                        },
                        v,
                    )
                }),
        )
    }
    pub fn keys<V: 'static>(&self) -> Option<impl Iterator<Item = SceneKey<V>> + '_> {
        let ty_id = TypeId::of::<V>();
        Some(
            self.data
                .get(&ty_id)?
                .downcast_ref::<SlotMap<DefaultKey, V>>()?
                .keys()
                .map(|k| SceneKey {
                    key: k,
                    _ty: PhantomData,
                }),
        )
    }
    pub fn values<V: 'static>(&self) -> Option<impl Iterator<Item = &V>> {
        let ty_id = TypeId::of::<V>();
        Some(
            self.data
                .get(&ty_id)?
                .downcast_ref::<SlotMap<DefaultKey, V>>()?
                .values(),
        )
    }
    pub fn values_mut<V: 'static>(&mut self) -> Option<impl Iterator<Item = &mut V>> {
        let ty_id = TypeId::of::<V>();
        Some(
            self.data
                .get_mut(&ty_id)?
                .downcast_mut::<SlotMap<DefaultKey, V>>()?
                .values_mut(),
        )
    }

    pub fn new() -> Self {
        Self {
            geometries: SlotMap::new(),
            blases: SlotMap::new(),
            instances: SlotMap::new(),
            tlas: None,
            data: HashMap::new(),
        }
    }
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        for (_, blas) in self.blases.iter() {
            blas.build(self, cache, rgraph);
        }
        self.tlas.as_ref().unwrap().build(self, cache, rgraph);
    }
    pub fn load(&mut self, device: &Arc<Device>) {
        let (models, materials, ..) = load_obj_buf(
            &mut BufReader::new(include_bytes!("res/onecube_scene.obj").as_slice()),
            &GPU_LOAD_OPTIONS,
            |_| {
                load_mtl_buf(&mut BufReader::new(
                    include_bytes!("res/onecube_scene.mtl").as_slice(),
                ))
            },
        )
        .unwrap();

        for model in models.iter() {
            self.insert(BlasGeometry::create(
                device,
                &model.mesh.indices,
                &model.mesh.positions,
            ));
        }

        for geometry in self.iter::<BlasGeometry>().unwrap() {
            self.insert(Blas::create(device, geometry));
        }

        for key in self.keys().unwrap() {
            self.instances.insert(BlasInstance {
                blas: key,
                transform: vk::TransformMatrixKHR {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, //
                        0.0, 1.0, 0.0, 0.0, //
                        0.0, 0.0, 1.0, 0.0, //
                    ],
                },
                instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                ),
            });
        }

        self.tlas = Some(Tlas::create(
            device,
            self,
            self.instances.iter().collect::<Vec<_>>(),
        ));
    }
}
