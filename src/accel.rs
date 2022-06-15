use bevy_ecs::prelude::*;
use screen_13::prelude::*;
use slotmap::DefaultKey;
use std::sync::Arc;

use crate::world::{BlasKey, GeometryKey, InstanceKey, MaterialKey, Scene};

use super::buffers::*;

#[derive(Component)]
pub struct BlasGeometry {
    positions: Arc<PositionsBuffer>,
    indices: Arc<IndexBuffer>,
}

impl BlasGeometry {
    pub fn create(device: &Arc<Device>, indices: &[u32], positions: &[f32]) -> Self {
        let positions = Arc::new(PositionsBuffer::create(device, positions));
        let indices = Arc::new(IndexBuffer::create(device, indices));

        Self { positions, indices }
    }
}

pub struct Blas {
    geometry: usize,
    pub accel: Arc<AccelerationStructure>,
    geometry_info: AccelerationStructureGeometryInfo,
    size: AccelerationStructureSize,
}

impl Blas {
    pub fn build(&self, scene: &Scene, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        let geometry = scene.geometries.get(self.geometry).unwrap();
        let index_node = rgraph.bind_node(&geometry.indices.data);
        let vertex_node = rgraph.bind_node(&geometry.positions.data);
        let accel_node = rgraph.bind_node(&self.accel);

        let scratch_buf = rgraph.bind_node(
            cache
                .lease(BufferInfo::new(
                    self.size.build_size,
                    vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ))
                .unwrap(),
        );

        let triangle_count = geometry.indices.count / 3;
        let geometry_info = self.geometry_info.clone();

        rgraph
            .begin_pass("Build BLAS")
            .read_node(index_node)
            .read_node(vertex_node)
            .write_node(accel_node)
            .write_node(scratch_buf)
            .record_acceleration(move |accel| {
                accel.build_structure(
                    accel_node,
                    scratch_buf,
                    geometry_info,
                    &[vk::AccelerationStructureBuildRangeInfoKHR {
                        first_vertex: 0,
                        primitive_count: triangle_count as u32,
                        primitive_offset: 0,
                        transform_offset: 0,
                    }],
                )
            });
    }
    pub fn create(device: &Arc<Device>, (gkey, geometry): (usize, &BlasGeometry)) -> Self {
        let triangle_count = geometry.indices.count / 3;
        let vertex_count = geometry.positions.count / 3;

        let geometry_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![AccelerationStructureGeometry {
                max_primitive_count: triangle_count as _,
                flags: vk::GeometryFlagsKHR::OPAQUE,
                geometry: AccelerationStructureGeometryData::Triangles {
                    index_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &geometry.indices.data,
                    )),
                    index_type: geometry.indices.ty,
                    transform_data: None,
                    max_vertex: vertex_count as _,
                    vertex_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &geometry.positions.data,
                    )),
                    vertex_format: geometry.positions.format,
                    vertex_stride: geometry.positions.stride as _,
                },
            }],
        };

        let accel_size = AccelerationStructure::size_of(device, &geometry_info);

        let accel_info = AccelerationStructureInfo {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            size: accel_size.create_size,
        };

        let accel = AccelerationStructure::create(device, accel_info).unwrap();
        Self {
            geometry: gkey,
            accel: Arc::new(accel),
            geometry_info,
            size: accel_size,
        }
    }
}

pub struct BlasInstance {
    pub blas: usize,
    // TODO: add shader references.
    pub material: usize,
    pub shader: usize,
    pub transform: vk::TransformMatrixKHR,
}

impl BlasInstance {
    pub fn to_vk(&self, scene: &Scene) -> vk::AccelerationStructureInstanceKHR {
        // TODO: Maybee we should not use SlotMaps. Or find a better way to get the indices of the
        // materials.
        vk::AccelerationStructureInstanceKHR {
            transform: self.transform,
            instance_custom_index_and_mask: vk::Packed24_8::new(self.material as _, 0xff),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                0,
                vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
            ),
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: AccelerationStructure::device_address(
                    &scene.blases.get(self.blas).unwrap().accel,
                ),
            },
        }
    }
}

pub struct Material {
    pub diffuse: [f32; 4],
    pub mra: [f32; 4],
}

impl Material {
    pub fn to_vk(&self, scene: &Scene) -> VkMaterial {
        VkMaterial {
            diffuse: self.diffuse,
            mra: self.mra,
        }
    }
}

pub struct Tlas {
    instance_buf: InstanceBuffer,
    pub material_buf: MaterialBuffer,
    pub accel: Arc<AccelerationStructure>,
    geometry_info: AccelerationStructureGeometryInfo,
    size: AccelerationStructureSize,
}

impl Tlas {
    pub fn build(&self, scene: &Scene, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        let scratch_buf = rgraph.bind_node(
            cache
                .lease(BufferInfo::new(
                    self.size.build_size,
                    vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ))
                .unwrap(),
        );
        let accel_node = rgraph.bind_node(&self.accel);
        let instance_node = rgraph.bind_node(&self.instance_buf.data);
        let tlas_node = rgraph.bind_node(&self.accel);
        let geometry_info = self.geometry_info.clone();
        let primitive_count = scene.instances.len();

        // TODO: this is only necesarry to generate blases before tlas.
        let blas_nodes = scene
            .blases
            .iter()
            .map(|b| rgraph.bind_node(&b.accel))
            .collect::<Vec<_>>();

        let mut pass = rgraph.begin_pass("build TLAS").read_node(instance_node);
        for blas_node in blas_nodes {
            pass = pass.read_node(blas_node);
        }
        pass.write_node(scratch_buf)
            .write_node(tlas_node)
            .record_acceleration(move |accel| {
                accel.build_structure(
                    accel_node,
                    scratch_buf,
                    geometry_info,
                    &[vk::AccelerationStructureBuildRangeInfoKHR {
                        primitive_count: primitive_count as _,
                        primitive_offset: 0,
                        first_vertex: 0,
                        transform_offset: 0,
                    }],
                );
            });
    }
    pub fn create(device: &Arc<Device>, scene: &Scene) -> Self {
        let instances = scene
            .instances
            .iter()
            .map(|i| i.to_vk(scene))
            .collect::<Vec<_>>();
        let instance_buf = InstanceBuffer::create(device, &instances);
        let materials = scene
            .materials
            .iter()
            .map(|m| m.to_vk(scene))
            .collect::<Vec<_>>();
        let material_buf = MaterialBuffer::create(device, &materials);
        let geometry_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![AccelerationStructureGeometry {
                max_primitive_count: instances.len() as _,
                flags: vk::GeometryFlagsKHR::OPAQUE,
                geometry: AccelerationStructureGeometryData::Instances {
                    array_of_pointers: false,
                    data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &instance_buf.data,
                    )),
                },
            }],
        };

        let size = AccelerationStructure::size_of(device, &geometry_info);

        let info = AccelerationStructureInfo {
            ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            size: size.create_size,
        };

        let accel = Arc::new(AccelerationStructure::create(device, info).unwrap());

        Self {
            instance_buf,
            material_buf,
            size,
            geometry_info,
            accel,
        }
    }
}
