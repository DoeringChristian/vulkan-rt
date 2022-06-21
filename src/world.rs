use crate::accel::{Blas, BlasGeometry, BlasInstance, Material, Tlas};
use crate::buffers::{GlslInstanceData, GlslMaterial};
use crate::model::Model;

use bevy_ecs::prelude::*;
use screen_13::prelude::*;
use slotmap::*;
use std::collections::{BTreeMap, HashMap};
use std::io::BufReader;
use std::sync::Arc;

pub struct GpuScene {
    //pub world: World,
    //pub geometries: Vec<BlasGeometry>,
    pub blases: Vec<Blas>,
    pub tlas: Tlas,
}

impl GpuScene {
    pub fn create(device: &Arc<Device>, scene: &mut Scene) -> Self {
        let geometries = scene
            .world
            .query::<(Entity, &Model)>()
            .iter(&scene.world)
            .enumerate()
            .map(|(i, (e, m))| {
                (
                    e,
                    (i, BlasGeometry::create(device, &m.indices, &m.positions)),
                )
            })
            .collect::<HashMap<_, _>>();

        let blases = geometries
            .into_iter()
            .map(|(e, (i, g))| (e, (i, Blas::create(device, g))))
            .collect::<BTreeMap<_, _>>();
        let mut instances = vec![];
        let mut materials = vec![];
        let mut instancedata = vec![];
        for (i, (entity, instance)) in scene
            .world
            .query::<(Entity, &BlasInstance)>()
            .iter(&scene.world)
            .enumerate()
        {
            let material: &Material = scene
                .world
                .get_entity(instance.material)
                .unwrap()
                .get()
                .unwrap();
            materials.push(GlslMaterial {
                diffuse: material.diffuse,
                mra: material.mra,
                emission: [
                    material.emission[0],
                    material.emission[1],
                    material.emission[2],
                    0.,
                ],
            });
            instancedata.push(GlslInstanceData {
                mat_index: (materials.len() - 1) as _,
                model: blases[&instance.model].0 as _,
                //_pad: [0, 0],
            });
            instances.push(vk::AccelerationStructureInstanceKHR {
                transform: instance.transform,
                instance_custom_index_and_mask: vk::Packed24_8::new(
                    (instancedata.len() - 1) as _,
                    0xff,
                ),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                ),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: AccelerationStructure::device_address(
                        &blases[&instance.model].1.accel,
                    ),
                },
            });
        }
        trace!("instances: {}", instancedata.len());

        // TODO: very convoluted need better way.
        let mut blases = blases
            .into_iter()
            .map(|(_, (i, b))| (i, b))
            .collect::<Vec<_>>();
        blases.sort_by(|(i0, _), (i1, _)| i0.cmp(i1));
        let blases = blases.into_iter().map(|(i, b)| b).collect::<Vec<_>>();
        let tlas = Tlas::create(device, &instancedata, &instances, &materials);

        Self { blases, tlas }
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
    pub world: World,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }
    pub fn load_gltf(&mut self, device: &Arc<Device>) {
        let (gltf, buffers, _) = gltf::import("./src/res/room.gltf").unwrap();
        {
            gltf.meshes().for_each(|mesh| {
                let primitive = mesh.primitives().next().unwrap();
                let material = primitive.material();
                let mr = material.pbr_metallic_roughness();
                let emission = material.emissive_factor();
                let material_id = self
                    .world
                    .spawn()
                    .insert(Material {
                        diffuse: mr.base_color_factor(),
                        mra: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                        emission,
                    })
                    .id();
                let mut model = Model {
                    indices: Vec::new(),
                    positions: Vec::new(),
                    //uvs: Vec::new(),
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
                let model = self.world.spawn().insert(model).id();
                self.world.spawn().insert(BlasInstance {
                    model,
                    material: material_id,
                    transform: vk::TransformMatrixKHR {
                        matrix: [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0.],
                    },
                });
            });
        }
    }
}
