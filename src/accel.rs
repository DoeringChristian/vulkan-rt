use bevy_ecs::prelude::*;
use screen_13::prelude::*;
use slotmap::DefaultKey;
use std::{collections::HashMap, sync::Arc};

use crate::{
    model::{Index, Position},
    world::{GpuScene, Scene},
};

use super::buffers::*;

#[derive(Component)]
pub struct BlasGeometry {
    //pub positions: Arc<PositionsBuffer>,
    //pub indices: Arc<IndexBuffer>,
    pub positions: TypedBuffer<[f32; 3]>,
    pub indices: TypedBuffer<u32>,
}

impl BlasGeometry {
    pub fn create(device: &Arc<Device>, indices: &[u32], positions: &[[f32; 3]]) -> Self {
        //let positions = Arc::new(PositionsBuffer::create(device, positions));
        //let indices = Arc::new(IndexBuffer::create(device, indices));
        let positions = TypedBuffer::create(
            device,
            positions,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
        );
        let indices = TypedBuffer::create(
            device,
            indices,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::STORAGE_BUFFER,
        );

        Self { positions, indices }
    }
}

pub struct Blas {
    pub accel: Arc<AccelerationStructure>,
    pub geometry: BlasGeometry,
    geometry_info: AccelerationStructureGeometryInfo,
    size: AccelerationStructureSize,
}

impl Blas {
    pub fn build(
        &self,
        scene: &GpuScene,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) -> AnyAccelerationStructureNode {
        //let geometry = scene.geometries.get(self.geometry).unwrap();
        let index_node = rgraph.bind_node(&self.geometry.indices.buf);
        let vertex_node = rgraph.bind_node(&self.geometry.positions.buf);
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

        let triangle_count = self.geometry.indices.count() / 3;
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
        AnyAccelerationStructureNode::AccelerationStructure(accel_node)
    }
    pub fn create(device: &Arc<Device>, geometry: BlasGeometry) -> Self {
        let triangle_count = geometry.indices.count() / 3;
        let vertex_count = geometry.positions.count();

        let geometry_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![AccelerationStructureGeometry {
                max_primitive_count: triangle_count as _,
                flags: vk::GeometryFlagsKHR::OPAQUE,
                geometry: AccelerationStructureGeometryData::Triangles {
                    index_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &geometry.indices.buf,
                    )),
                    index_type: vk::IndexType::UINT32,
                    transform_data: None,
                    max_vertex: vertex_count as _,
                    vertex_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &geometry.positions.buf,
                    )),
                    vertex_format: vk::Format::R32G32B32_SFLOAT,
                    vertex_stride: std::mem::size_of::<[f32; 3]>() as _,
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
            //geometry: gkey,
            accel: Arc::new(accel),
            geometry,
            geometry_info,
            size: accel_size,
        }
    }
}

#[derive(Component)]
pub struct Material {
    pub diffuse: [f32; 4],
    pub mra: [f32; 4],
    pub emission: [f32; 3],
}

/*
impl Material {
    pub fn to_vk(&self, scene: &Scene) -> GlslMaterial {
        GlslMaterial {
            diffuse: self.diffuse,
            mra: self.mra,
        }
    }
}
*/

pub struct Tlas {
    instance_buf: InstanceBuffer,
    pub material_buf: MaterialBuffer,
    pub accel: Arc<AccelerationStructure>,
    pub instancedata_buf: InstanceDataBuf,
    geometry_info: AccelerationStructureGeometryInfo,
    size: AccelerationStructureSize,
}

impl Tlas {
    pub fn build(
        &self,
        scene: &GpuScene,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
        blas_nodes: &[AnyAccelerationStructureNode],
    ) {
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
        let primitive_count = scene.blases.len();

        // TODO: this is only necesarry to generate blases before tlas.
        /*let blas_nodes = scene
        .blases
        .iter()
        .map(|b| rgraph.bind_node(&b.accel))
        .collect::<Vec<_>>();*/

        let mut pass = rgraph.begin_pass("build TLAS");
        for blas_node in blas_nodes {
            pass = pass.read_node(*blas_node);
        }
        //pass.read_node(instance_node)
        pass.access_node(instance_node, AccessType::AccelerationStructureBuildRead)
            .write_node(scratch_buf)
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
    pub fn create(
        device: &Arc<Device>,
        instances_data: &[GlslInstanceData],
        instances: &[vk::AccelerationStructureInstanceKHR],
        materials: &[GlslMaterial],
    ) -> Self {
        // gl_CustomIndexEXT should index into attributes.
        let instancedata_buf = InstanceDataBuf::create(device, &instances_data);
        let instance_buf = InstanceBuffer::create(device, &instances);
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
            instancedata_buf,
            instance_buf,
            material_buf,
            size,
            geometry_info,
            accel,
        }
    }
}
