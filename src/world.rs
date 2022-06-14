use crate::accel::{Blas, BlasGeometry, BlasInstance, Material, Tlas};

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
    pub fn render(
        &self,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
        image: impl Into<AnyImageNode>,
    ) {
        let blas_nodes = self
            .blases
            .values()
            .map(|b| rgraph.bind_node(&b.accel))
            .collect::<Vec<_>>();
        let material_node = rgraph.bind_node(&self.tlas.as_ref().unwrap().material_buf.data);
        let tlas_ndoe = rgraph.bind_node(&self.tlas.as_ref().unwrap().accel);
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

        let material_keys = materials
            .unwrap()
            .into_iter()
            .map(|m| {
                self.materials.insert(Material {
                    diffuse: [m.diffuse[0], m.diffuse[1], m.diffuse[2], 1.],
                })
            })
            .collect::<Vec<_>>();

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

        // create a instance for every blas.
        for (i, key) in self.blases.keys().enumerate() {
            self.instances.insert(BlasInstance {
                blas: key,
                // TODO: better material indexing / instance loading
                material: material_keys[models[i].mesh.material_id.unwrap_or_default()],
                transform: vk::TransformMatrixKHR {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, //
                        0.0, 1.0, 0.0, 0.0, //
                        0.0, 0.0, 1.0, 0.0, //
                    ],
                },
            });
        }

        self.tlas = Some(Tlas::create(device, self));
    }
}
