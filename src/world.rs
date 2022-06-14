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

// TODO: change add pipeline and sbt to scene.
// where sbt indexes into shader groups of pipeline.
// (maybee sbt should be part of tlas)
pub struct Scene {
    pub geometries: Vec<BlasGeometry>,
    pub blases: Vec<Blas>,
    pub instances: Vec<BlasInstance>,
    pub materials: Vec<Material>,
    pub tlas: Option<Tlas>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            geometries: Vec::new(),
            blases: Vec::new(),
            instances: Vec::new(),
            materials: Vec::new(),
            tlas: None,
        }
    }
    /*
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
    */
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        for blas in self.blases.iter() {
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
                self.materials.push(Material {
                    diffuse: [m.diffuse[0], m.diffuse[1], m.diffuse[2], 1.],
                });
                self.materials.len() - 1
            })
            .collect::<Vec<_>>();

        for model in models.iter() {
            self.geometries.push(BlasGeometry::create(
                device,
                &model.mesh.indices,
                &model.mesh.positions,
            ));
        }

        for geometry in self.geometries.iter().enumerate() {
            self.blases.push(Blas::create(device, geometry));
        }

        // create a instance for every blas.
        for (i, m) in models.iter().enumerate() {
            self.instances.push(BlasInstance {
                blas: i,
                material: material_keys[m.mesh.material_id.unwrap_or_default()],
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
