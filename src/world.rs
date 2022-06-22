use crate::accel::{Blas, BlasGeometry, BlasInfo, Material, Tlas};
use crate::buffers::TypedBuffer;
use crate::model::{
    GlslInstanceData, GlslMaterial, Index, InstanceBundle, MaterialId, MeshId, Position, VertexData,
};

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

    pub material_buf: TypedBuffer<GlslMaterial>,
    pub instancedata_buf: TypedBuffer<GlslInstanceData>,
    pub positions_bufs: Vec<Arc<TypedBuffer<Position>>>,
    pub indices_bufs: Vec<Arc<TypedBuffer<Index>>>,
}

impl GpuScene {
    pub fn create(device: &Arc<Device>, scene: &mut Scene) -> Self {
        let mut positions_bufs = vec![];
        let mut indices_bufs = vec![];
        let mut blases = vec![];
        let mut mesh_idxs = HashMap::new();
        struct MeshIdxs {
            positions: usize,
            indices: usize,
            blas: usize,
        }
        for (entity, positions, indices) in scene
            .world
            .query::<(Entity, &VertexData<Position>, &VertexData<Index>)>()
            .iter(&scene.world)
        {
            mesh_idxs.insert(
                entity,
                MeshIdxs {
                    positions: positions_bufs.len(),
                    indices: indices_bufs.len(),
                    blas: blases.len(),
                },
            );
            positions_bufs.push(Arc::new(TypedBuffer::create(
                device,
                &positions.0,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            )));
            indices_bufs.push(Arc::new(TypedBuffer::create(
                device,
                &indices.0,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            )));
            blases.push(Blas::create(
                device,
                &BlasInfo {
                    indices: indices_bufs.last().unwrap(),
                    positions: positions_bufs.last().unwrap(),
                },
            ));
        }
        let mut materials = vec![];
        let mut material_idxs = HashMap::new();
        for (entity, material) in scene
            .world
            .query::<(Entity, &Material)>()
            .iter(&scene.world)
        {
            material_idxs.insert(entity, materials.len());
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
        }
        let mut instances = vec![];
        //let mut materials = vec![];
        let mut instancedata = vec![];
        for (entity, material_id, mesh_id, transform) in scene
            .world
            .query::<(Entity, &MaterialId, &MeshId, &Transform)>()
            .iter(&scene.world)
        {
            instancedata.push(GlslInstanceData {
                mat_index: material_idxs[&material_id.0] as _,
                positions: mesh_idxs[&mesh_id.0].positions as _,
                indices: mesh_idxs[&mesh_id.0].indices as _,
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
                        &blases[mesh_idxs[&mesh_id.0].blas].accel,
                    ),
                },
            });
        }
        trace!("instances: {}", instancedata.len());

        let material_buf =
            TypedBuffer::create(device, &materials, vk::BufferUsageFlags::STORAGE_BUFFER);
        let instancedata_buf =
            TypedBuffer::create(device, &instancedata, vk::BufferUsageFlags::STORAGE_BUFFER);

        let tlas = Tlas::create(device, &instances);

        Self {
            blases,
            tlas,
            material_buf,
            instancedata_buf,
            positions_bufs,
            indices_bufs,
        }
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
            let mut mesh_entities = HashMap::new();
            for mesh in gltf.meshes() {
                let primitive = mesh.primitives().next().unwrap();
                let mut indices: VertexData<Index> = VertexData(Vec::new());
                let mut positions: VertexData<Position> = VertexData(Vec::new());
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(iter) = reader.read_positions() {
                    for position in iter {
                        positions.0.push(Position(position));
                    }
                }
                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        indices.0.push(Index(index))
                    }
                }
                let entity = self.world.spawn().insert(indices).insert(positions).id();
                mesh_entities.insert(mesh.index(), entity);
            }
            let mut material_entities = HashMap::new();
            for material in gltf.materials() {
                let mr = material.pbr_metallic_roughness();
                let emission = material.emissive_factor();
                let material_entity = self
                    .world
                    .spawn()
                    .insert(Material {
                        diffuse: mr.base_color_factor(),
                        mra: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                        emission,
                    })
                    .id();
                material_entities.insert(material.index().unwrap(), material_entity);
            }
            for node in gltf.nodes() {
                if let Some(mesh) = node.mesh() {
                    self.world.spawn().insert_bundle(InstanceBundle {
                        mesh: MeshId(mesh_entities[&mesh.index()]),
                        material: MaterialId(
                            material_entities[&mesh
                                .primitives()
                                .next()
                                .unwrap()
                                .material()
                                .index()
                                .unwrap()],
                        ),
                        transform: Transform::from_xyz(0., 0., 0.),
                    });
                }
            }
        }
    }
}
