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

#[derive(Clone, Copy)]
pub struct SceneIndex<T> {
    _ty: PhantomData<T>,
    key: DefaultKey,
}

pub struct Scene {
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
        }
    }

    pub fn insert<T: 'static>(&mut self, val: T) -> SceneIndex<T> {
        let ty_id = TypeId::of::<T>();

        let key = self
            .map
            .entry(ty_id)
            .or_insert(Box::new(SlotMap::<DefaultKey, T>::new()))
            .downcast_mut::<SlotMap<DefaultKey, T>>()
            .unwrap()
            .insert(val);

        SceneIndex {
            _ty: PhantomData,
            key,
        }
    }
    pub fn get<T: 'static>(&self, index: SceneIndex<T>) -> Option<&T> {
        let ty_id = TypeId::of::<T>();
        Some(
            self.map
                .get(&ty_id)?
                .downcast_ref::<SlotMap<DefaultKey, T>>()?
                .get(index.key)?,
        )
    }
    pub fn get_mut<T: 'static>(&mut self, index: SceneIndex<T>) -> Option<&mut T> {
        let ty_id = TypeId::of::<T>();
        Some(
            self.map
                .get_mut(&ty_id)?
                .downcast_mut::<SlotMap<DefaultKey, T>>()?
                .get_mut(index.key)?,
        )
    }
}

pub struct GpuWorld {
    pub geometries: Vec<Arc<BlasGeometry>>,
    pub blases: Vec<Arc<Blas>>,
    pub instances: Arc<Vec<BlasInstance>>,
    pub tlas: Arc<Tlas>,
}

impl GpuWorld {
    pub fn update_tlas(
        &self,
        device: &Arc<Device>,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) {
        //self.tlas.update_instance_buf(device, &self.instances);
        self.tlas.update(cache, rgraph);
    }
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        for blas in self.blases.iter() {
            blas.build(cache, rgraph);
        }
        self.tlas.build(cache, rgraph);
    }
    pub fn load(device: &Arc<Device>) -> Self {
        let mut rgraph = RenderGraph::new();
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

        let geometries = models
            .into_iter()
            .map(|m| {
                Arc::new(BlasGeometry::create(
                    device,
                    &m.mesh.indices,
                    &m.mesh.positions,
                ))
            })
            .collect::<Vec<_>>();

        let blas = geometries
            .iter()
            .map(|g| Arc::new(Blas::create(device, &g)))
            .collect::<Vec<_>>();

        let instances = Arc::new(
            blas.iter()
                .map(|blas| BlasInstance {
                    blas: blas.clone(),
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
                })
                .collect::<Vec<_>>(),
        );

        let tlas = Arc::new(Tlas::create(device, &instances));

        Self {
            geometries,
            blases: blas,
            tlas,
            instances,
        }
    }
}
