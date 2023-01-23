use crevice::std140::{AsStd140, Std140};
use screen_13::prelude::*;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::Range;
use std::{collections::HashMap, sync::Arc};

use crate::array::Array;

pub struct Blas<T> {
    pub accel: Arc<AccelerationStructure>,
    pub indices: Arc<Buffer>,
    pub positions: Arc<Buffer>,
    primitive_count: usize,
    geometry_info: AccelerationStructureGeometryInfo,
    size: AccelerationStructureSize,
    primitive_offset: usize,
    _ty: PhantomData<T>,
}

impl<T: AsStd140> Blas<T> {
    pub fn build(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        //let geometry = scene.geometries.get(self.geometry).unwrap();
        let indices = self.indices.clone();
        let positions = self.positions.clone();
        let index_node = rgraph.bind_node(&indices);
        let vertex_node = rgraph.bind_node(&positions);
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

        let triangle_count = self.primitive_count;
        let geometry_info = self.geometry_info.clone();
        let primitive_offset = self.primitive_offset;
        let primitive_count = self.primitive_count;

        rgraph
            .begin_pass("Build BLAS")
            .read_node(index_node)
            .read_node(vertex_node)
            .write_node(accel_node)
            .write_node(scratch_buf)
            .record_acceleration(move |accel, _| {
                accel.build_structure(
                    accel_node,
                    scratch_buf,
                    &geometry_info,
                    &[vk::AccelerationStructureBuildRangeInfoKHR {
                        first_vertex: 0,
                        primitive_count: primitive_count as u32,
                        primitive_offset: 0,
                        transform_offset: 0,
                    }],
                )
            });
        //AnyAccelerationStructureNode::AccelerationStructure(accel_node)
    }
    pub fn create(
        device: &Arc<Device>,
        indices: &Array<u32>,
        indices_offset: usize,
        primitive_count: usize,
        vertices: &Array<T>,
        vertices_offset: usize,
    ) -> Self {
        //let triangle_count = geometry.indices.count() / 3;
        let vertex_count = vertices.count() as u64;
        let vertex_stride = vertices.stride() as u64;
        //let vertex_count = geometry.positions.count();

        let geometry_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![AccelerationStructureGeometry {
                max_primitive_count: primitive_count as _,
                flags: vk::GeometryFlagsKHR::OPAQUE,
                geometry: AccelerationStructureGeometryData::Triangles {
                    index_data: DeviceOrHostAddress::DeviceAddress(
                        screen_13::prelude::Buffer::device_address(&indices.buf)
                            + (indices_offset * indices.stride()) as u64,
                    ),
                    index_type: vk::IndexType::UINT32,
                    transform_data: None,
                    max_vertex: vertex_count as _,
                    vertex_data: DeviceOrHostAddress::DeviceAddress(
                        screen_13::prelude::Buffer::device_address(&vertices.buf)
                            + (vertices_offset * vertices.stride()) as u64,
                    ),
                    vertex_format: vk::Format::R32G32B32_SFLOAT,
                    vertex_stride: vertex_stride,
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
            indices: indices.buf.clone(),
            positions: vertices.buf.clone(),
            geometry_info,
            primitive_count,
            size: accel_size,
            primitive_offset: indices_offset,
            _ty: PhantomData::default(),
        }
    }
}

pub struct Tlas {
    instance_buf: Arc<Array<u8>>,
    pub accel: Arc<AccelerationStructure>,
    //pub instancedata_buf: TypedBuffer<GlslInstanceData>,
    geometry_info: AccelerationStructureGeometryInfo,
    size: AccelerationStructureSize,
    instance_count: usize,
}

impl Tlas {
    pub fn build(
        &self,
        //scene: &GpuScene,
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
        let instance_node = rgraph.bind_node(&self.instance_buf.buf);
        let tlas_node = rgraph.bind_node(&self.accel);
        let geometry_info = self.geometry_info.clone();
        //let primitive_count = scene.blases.len();
        let primitive_count = self.instance_count;

        let mut pass = rgraph.begin_pass("Build TLAS");
        for blas_node in blas_nodes {
            //pass = pass.read_node(*blas_node);
            pass = pass.access_node(*blas_node, AccessType::AccelerationStructureBuildRead);
        }
        //pass.read_node(instance_node)
        pass.read_node(instance_node)
            .write_node(scratch_buf)
            .write_node(tlas_node)
            .record_acceleration(move |accel, _| {
                accel.build_structure(
                    accel_node,
                    scratch_buf,
                    &geometry_info,
                    &[vk::AccelerationStructureBuildRangeInfoKHR {
                        primitive_count: primitive_count as _,
                        primitive_offset: 0,
                        first_vertex: 0,
                        transform_offset: 0,
                    }],
                );
            });
        //println!("pass: {:#?}", rgraph);
    }
    pub fn create(
        device: &Arc<Device>,
        //instances_data: &[GlslInstanceData],
        instances: &[vk::AccelerationStructureInstanceKHR],
        //materials: &[GlslMaterial],
    ) -> Option<Self> {
        if (instances.len() == 0) {
            return None;
        }
        // gl_CustomIndexEXT should index into attributes.
        let instance_buf = Arc::new(Array::from_slice_u8(
            device,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            AccelerationStructure::instance_slice(instances),
        ));
        let geometry_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![AccelerationStructureGeometry {
                max_primitive_count: instances.len() as _,
                flags: vk::GeometryFlagsKHR::OPAQUE,
                geometry: AccelerationStructureGeometryData::Instances {
                    array_of_pointers: false,
                    data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &instance_buf.buf,
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

        Some(Self {
            instance_buf,
            instance_count: instances.len(),
            size,
            geometry_info,
            accel,
        })
    }
}
