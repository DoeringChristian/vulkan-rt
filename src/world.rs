use crate::accel::{Blas, BlasGeometry, BlasInstance, Tlas};

use super::buffers;
use bevy_ecs::prelude::*;
use bytemuck::cast_slice;
use screen_13::prelude::*;
use slotmap::*;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};
use std::{io::BufReader, mem::size_of};
use tobj::*;

pub struct Scene {
    pub geometries: SlotMap<DefaultKey, BlasGeometry>,
    pub blases: SlotMap<DefaultKey, Blas>,
    pub instances: SlotMap<DefaultKey, BlasInstance>,
    pub tlas: Option<Tlas>,
    //pub geometries: Vec<Arc<BlasGeometry>>,
    //pub blases: Vec<Arc<Blas>>,
    //pub instances: Arc<Vec<BlasInstance>>,
    //pub tlas: Arc<Tlas>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            geometries: SlotMap::new(),
            blases: SlotMap::new(),
            instances: SlotMap::new(),
            tlas: None,
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
            self.geometries.insert(BlasGeometry::create(
                device,
                &model.mesh.indices,
                &model.mesh.positions,
            ));
        }

        for geometry in self.geometries.iter() {
            self.blases.insert(Blas::create(device, geometry));
        }

        for key in self.blases.keys() {
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
