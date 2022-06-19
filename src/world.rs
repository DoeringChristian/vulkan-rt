use crate::accel::{Blas, BlasGeometry, BlasInstance, Material, Tlas};
use crate::buffers::{GlslAttribute, GlslMaterial};
use crate::model::Model;

use bevy_ecs::prelude::*;
use screen_13::prelude::*;
use slotmap::*;
use std::collections::HashMap;
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
            .collect::<HashMap<_, _>>();
        let materials = scene
            .world
            .query::<(Entity, &Material)>()
            .iter(&scene.world)
            .enumerate()
            .map(|(i, (e, m))| {
                (
                    e,
                    (
                        i,
                        GlslMaterial {
                            diffuse: m.diffuse,
                            mra: m.mra,
                        },
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let attributes = scene
            .world
            .query::<(Entity, &BlasInstance)>()
            .iter(&scene.world)
            .enumerate()
            .map(|(i, (e, inst))| {
                (
                    e,
                    (
                        i,
                        GlslAttribute {
                            mat_index: materials[&inst.material].0 as _,
                            model: blases[&inst.model].0 as _,
                        },
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let instances = scene
            .world
            .query::<(Entity, &BlasInstance)>()
            .iter(&scene.world)
            .enumerate()
            .map(|(i, (e, inst))| {
                (
                    e,
                    (
                        i,
                        vk::AccelerationStructureInstanceKHR {
                            transform: inst.transform,
                            instance_custom_index_and_mask: vk::Packed24_8::new(i as _, 0xff),
                            instance_shader_binding_table_record_offset_and_flags:
                                vk::Packed24_8::new(
                                    0,
                                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE
                                        .as_raw() as _,
                                ),
                            acceleration_structure_reference:
                                vk::AccelerationStructureReferenceKHR {
                                    device_handle: AccelerationStructure::device_address(
                                        &blases[&inst.model].1.accel,
                                    ),
                                },
                        },
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        let materials = materials
            .into_iter()
            .map(|(e, (i, m))| m)
            .collect::<Vec<_>>();
        let attributes = attributes
            .into_iter()
            .map(|(e, (i, a))| a)
            .collect::<Vec<_>>();
        let instances = instances
            .into_iter()
            .map(|(e, (i, inst))| inst)
            .collect::<Vec<_>>();
        let blases = blases.into_iter().map(|(e, (i, b))| b).collect::<Vec<_>>();
        let tlas = Tlas::create(device, &attributes, &instances, &materials);

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
        let (gltf, buffers, _) = gltf::import("./src/res/monkey.gltf").unwrap();
        {
            let materials = gltf
                .materials()
                .map(|material| {
                    let mr = material.pbr_metallic_roughness();
                    (
                        material.index().unwrap(),
                        self.world
                            .spawn()
                            .insert(Material {
                                diffuse: mr.base_color_factor(),
                                mra: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                            })
                            .id(),
                    )
                })
                .collect::<HashMap<_, _>>();
            let models = gltf
                .meshes()
                .map(|mesh| {
                    let primitive = mesh.primitives().next().unwrap();
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
                    (
                        self.world.spawn().insert(model).id(),
                        primitive.material().index().unwrap(),
                    )
                })
                .collect::<Vec<_>>();
            for (model, mat_id) in models {
                self.world.spawn().insert(BlasInstance {
                    model,
                    material: materials[&mat_id],
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
}
