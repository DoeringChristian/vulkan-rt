use crate::accel::{Blas, BlasGeometry, Material, Tlas};
use crate::buffers::{GlslInstanceData, GlslMaterial};
use crate::model::{AsSlice, Index, InstanceBundle, MaterialId, MeshId, Position, VertexData};

use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;
use bytemuck::cast_slice;
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
            .query::<(Entity, &VertexData<Position>, &VertexData<Index>)>()
            .iter(&scene.world)
            .enumerate()
            .map(|(i, (e, positions, indices))| {
                (
                    e,
                    (
                        i,
                        BlasGeometry::create(device, indices.as_slice(), positions.as_slice()),
                    ),
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
        for (i, (entity, material_id, mesh_id, transform)) in scene
            .world
            .query::<(Entity, &MaterialId, &MeshId, &Transform)>()
            .iter(&scene.world)
            .enumerate()
        {
            let material: &Material = scene
                .world
                .get_entity(material_id.0)
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
                model: blases[&mesh_id.0].0 as _,
                //_pad: [0, 0],
            });
            let matrix = transform.compute_matrix();
            let matrix = [
                matrix.x_axis.x,
                matrix.y_axis.x,
                matrix.z_axis.x,
                matrix.w_axis.x,
                matrix.x_axis.y,
                matrix.y_axis.y,
                matrix.z_axis.y,
                matrix.w_axis.y,
                matrix.x_axis.z,
                matrix.y_axis.z,
                matrix.z_axis.z,
                matrix.w_axis.z,
            ];
            instances.push(vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR { matrix },
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
                        &blases[&mesh_id.0].1.accel,
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
        let (gltf, buffers, _) = gltf::import("./src/res/cube_scene.gltf").unwrap();
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
                let mut indices: VertexData<Index> = VertexData(Vec::new());
                let mut positions: VertexData<Position> = VertexData(Vec::new());
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(iter) = reader.read_positions() {
                    for position in iter {
                        /*
                        positions.0.push(position[0]);
                        positions.0.push(position[1]);
                        positions.0.push(position[2]);
                        */
                        positions.0.push(Position(position));
                    }
                }
                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        indices.0.push(Index(index))
                    }
                }
                //let model = self.world.spawn().insert(model).id();
                let mesh = self.world.spawn().insert(indices).insert(positions).id();
                self.world.spawn().insert_bundle(InstanceBundle {
                    mesh: MeshId(mesh),
                    material: MaterialId(material_id),
                    transform: Transform::from_xyz(0., 0., 0.),
                });
            });
        }
    }
}
