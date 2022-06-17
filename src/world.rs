use crate::accel::{Blas, BlasGeometry, BlasInstance, Material, Tlas};
use crate::model::Model;

use screen_13::prelude::*;
use slotmap::*;
use std::collections::HashMap;
use std::io::BufReader;
use std::sync::Arc;

pub struct GpuScene {
    pub geometries: Vec<BlasGeometry>,
    pub blases: Vec<Blas>,
    pub tlas: Tlas,
}

impl GpuScene {
    pub fn create(device: &Arc<Device>, scene: &Scene) -> Self {
        let geometries = scene
            .models
            .iter()
            .map(|m| BlasGeometry::create(device, &m.indices, &m.positions))
            .collect::<Vec<_>>();

        let blases = geometries
            .iter()
            .enumerate()
            .map(|g| Blas::create(device, g))
            .collect::<Vec<_>>();
        // TODO: find better way. We could use the same index for blas, geometry and model if we
        // generate a geometry, blas for every model.
        let model_to_blas = blases.iter().map(|b| b).collect::<Vec<_>>();
        // create instance for each model TODO: load instances from gltf nodes.
        let tlas = Tlas::create(device, scene, &model_to_blas);

        Self {
            geometries,
            blases,
            tlas,
        }
    }
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        let blas_nodes = self
            .blases
            .iter()
            .map(|b| b.build(self, cache, rgraph))
            .collect::<Vec<_>>();
        /*
        for blas in self.blases.iter() {
            blas.build(self, cache, rgraph);
        }
        */
        self.tlas.build(self, cache, rgraph, &blas_nodes);
    }
}

// TODO: change add pipeline and sbt to scene.
// where sbt indexes into shader groups of pipeline.
// (maybee sbt should be part of tlas)
pub struct Scene {
    pub models: Vec<Model>,
    pub materials: Vec<Material>,
    //pub geometries: Vec<BlasGeometry>,
    //pub blases: Vec<Blas>,
    pub instances: Vec<BlasInstance>,
    //pub tlas: Option<Tlas>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            //geometries: Vec::new(),
            //blases: Vec::new(),
            instances: Vec::new(),
            materials: Vec::new(),
            //tlas: None,
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
    /*
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        for blas in self.blases.iter() {
            blas.build(self, cache, rgraph);
        }
        self.tlas.as_ref().unwrap().build(self, cache, rgraph);
    }
    */
    pub fn load_gltf(&mut self, device: &Arc<Device>) {
        let (gltf, buffers, _) = gltf::import("./src/res/monkey.gltf").unwrap();
        // Load to cpu
        {
            for material in gltf.materials() {
                let mr = material.pbr_metallic_roughness();
                self.materials.push(Material {
                    diffuse: mr.base_color_factor(),
                    mra: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                });
            }
            for mesh in gltf.meshes() {
                let primitive = mesh.primitives().next().unwrap();
                let mut model = Model {
                    indices: Vec::new(),
                    positions: Vec::new(),
                    uvs: Vec::new(),
                };
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(iter) = reader.read_positions() {
                    for position in iter {
                        model.positions.push(position[0]);
                        model.positions.push(position[1]);
                        model.positions.push(position[2]);
                    }
                }
                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        model.indices.push(index)
                    }
                }
                self.models.push(model);
            }
            self.instances.push(BlasInstance {
                model: 0,
                material: 0,
                shader: 0,
                transform: vk::TransformMatrixKHR {
                    matrix: [
                        1.0, 0.0, 0.0, 0.0, //
                        0.0, 1.0, 0.0, 0.0, //
                        0.0, 0.0, 1.0, 0.0, //
                    ],
                },
            });
        }
    }
}
