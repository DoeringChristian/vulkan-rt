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
                            emission: [m.emission[0], m.emission[1], m.emission[2], 0.],
                        },
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let (instances, materials) = scene
            .world
            .query::<(Entity, &BlasInstance)>()
            .iter(&scene.world)
            .enumerate()
            .map(|(i, (e, inst))| {
                // Add attribute and instance.
                //trace!("BlasInstance");
                let material: &Material = scene
                    .world
                    .get_entity(inst.material)
                    .unwrap()
                    .get()
                    .unwrap();
                (
                    vk::AccelerationStructureInstanceKHR {
                        transform: inst.transform,
                        instance_custom_index_and_mask: vk::Packed24_8::new(i as _, 0xff),
                        instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                            0,
                            vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw()
                                as _,
                        ),
                        acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                            device_handle: AccelerationStructure::device_address(
                                &blases[&inst.model].1.accel,
                            ),
                        },
                    },
                    GlslMaterial {
                        diffuse: material.diffuse,
                        mra: material.mra,
                        emission: [
                            material.emission[0],
                            material.emission[1],
                            material.emission[2],
                            0.,
                        ],
                    },
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        /*let materials = materials
        .into_iter()
        .map(|(e, (i, m))| m)
        .collect::<Vec<_>>();
        */
        let blases = blases.into_iter().map(|(e, (i, b))| b).collect::<Vec<_>>();
        let tlas = Tlas::create(device, &instances, &materials);

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
            /*
            let materials = gltf
                .materials()
                .map(|material| {
                    let mr = material.pbr_metallic_roughness();
                    let emission = material.emissive_factor();
                    (
                        material.index().unwrap(),
                        self.world
                            .spawn()
                            .insert(Material {
                                diffuse: mr.base_color_factor(),
                                mra: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                                emission,
                            })
                            .id(),
                    )
                })
                .collect::<HashMap<_, _>>();
                */
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
