use crate::accel::{Blas, BlasGeometry, BlasInstance, Tlas};
use crate::buffers::Material;

use screen_13::prelude::*;
use slotmap::*;
use std::io::BufReader;
use std::sync::Arc;
use tobj::*;

new_key_type! {
    pub struct GeometryKey;
    pub struct InstanceKey;
    pub struct BlasKey;
    pub struct MaterialKey;
}

pub struct Scene {
    pub geometries: SlotMap<GeometryKey, BlasGeometry>,
    pub blases: SlotMap<BlasKey, Blas>,
    pub instances: SlotMap<InstanceKey, BlasInstance>,
    pub materials: SlotMap<MaterialKey, Material>,
    pub tlas: Option<Tlas>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            geometries: SlotMap::default(),
            blases: SlotMap::default(),
            instances: SlotMap::default(),
            materials: SlotMap::default(),
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

        for material in materials.unwrap().iter() {
            self.materials.insert(Material {
                diffuse: [
                    material.diffuse[0],
                    material.diffuse[1],
                    material.diffuse[2],
                    1.,
                ],
            });
        }

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

        self.tlas = Some(Tlas::create(device, self));
    }
}
