use crate::accel::{Blas, BlasGeometry, BlasInstance, Tlas};

use super::buffers;
use bevy_ecs::prelude::*;
use bytemuck::cast_slice;
use screen_13::prelude::*;
use slotmap::*;
use std::sync::{Arc, Weak};
use std::{io::BufReader, mem::size_of};
use tobj::*;

/*
pub struct AccelerationStructureInfo{
    pub transform: [f32; 12],
    pub instance_custom_index: u32,
}
*/

#[derive(Component)]
pub struct GpuGeometry {
    pub vertices: Arc<Buffer>,
    pub indices: Arc<Buffer>,
    pub blas: Arc<AccelerationStructure>,
    blas_size: AccelerationStructureSize,
    geometry_info: AccelerationStructureGeometryInfo,
    triangle_count: usize,
    vertex_count: usize,
}

impl GpuGeometry {
    pub fn instance(&self) -> vk::AccelerationStructureInstanceKHR {
        vk::AccelerationStructureInstanceKHR {
            transform: vk::TransformMatrixKHR {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, //
                    0.0, 1.0, 0.0, 0.0, //
                    0.0, 0.0, 1.0, 0.0, //
                ],
            },
            instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                0,
                vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
            ),
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: AccelerationStructure::device_address(&self.blas),
            },
        }
    }

    pub fn build_blas(&self, rgraph: &mut RenderGraph, cache: &mut HashPool) {
        {
            let index_node = rgraph.bind_node(&self.indices);
            let vertex_node = rgraph.bind_node(&self.vertices);
            let blas_node = rgraph.bind_node(&self.blas);

            let scratch_buf = rgraph.bind_node(
                cache
                    .lease(BufferInfo::new(
                        self.blas_size.build_size,
                        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                            | vk::BufferUsageFlags::STORAGE_BUFFER,
                    ))
                    .unwrap(),
            );

            let triangle_count = self.triangle_count;
            let geometry_info = self.geometry_info.clone();

            rgraph
                .begin_pass("Build BLAS")
                .read_node(index_node)
                .read_node(vertex_node)
                .write_node(blas_node)
                .write_node(scratch_buf)
                .record_acceleration(move |accel| {
                    accel.build_structure(
                        blas_node,
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
    }
    pub fn from_tobj(device: &Arc<Device>, mesh: tobj::Mesh) -> Self {
        let triangle_count = mesh.indices.len() / 3;
        let vertex_count = mesh.positions.len() / 3;
        let indices = Arc::new({
            let data = cast_slice(&mesh.indices);
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(
                    data.len() as _,
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });
        let vertices = Arc::new({
            let data = cast_slice(&mesh.positions);
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(
                    data.len() as _,
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });

        let geometry_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![AccelerationStructureGeometry {
                max_primitive_count: triangle_count as _,
                flags: vk::GeometryFlagsKHR::OPAQUE,
                geometry: AccelerationStructureGeometryData::Triangles {
                    index_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &indices,
                    )),
                    index_type: vk::IndexType::UINT32,
                    max_vertex: vertex_count as _,
                    transform_data: None,
                    vertex_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                        &vertices,
                    )),
                    vertex_format: vk::Format::R32G32B32_SFLOAT,
                    vertex_stride: 12,
                },
            }],
        };
        let blas_size = AccelerationStructure::size_of(device, &geometry_info);
        let blas = Arc::new(
            AccelerationStructure::create(
                device,
                AccelerationStructureInfo {
                    ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                    size: blas_size.create_size,
                },
            )
            .unwrap(),
        );

        Self {
            vertices,
            indices,
            blas,
            blas_size,
            geometry_info,
            triangle_count,
            vertex_count,
        }
    }
}

pub struct GpuWorld {
    pub geometries: Vec<Arc<BlasGeometry>>,
    pub blases: Vec<Arc<Blas>>,
    pub instances: Arc<Vec<BlasInstance>>,
    pub tlas: Arc<Tlas>,
}

impl GpuWorld {
    pub fn update_tlas(
        &self,
        device: &Arc<Device>,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) {
        //self.tlas.update_instance_buf(device, &self.instances);
        self.tlas.update(cache, rgraph);
    }
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        for blas in self.blases.iter() {
            blas.build(cache, rgraph);
        }
        self.tlas.build(cache, rgraph);
    }
    pub fn load(device: &Arc<Device>) -> Self {
        let mut rgraph = RenderGraph::new();
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

        let geometries = models
            .into_iter()
            .map(|m| {
                Arc::new(BlasGeometry::create(
                    device,
                    &m.mesh.indices,
                    &m.mesh.positions,
                ))
            })
            .collect::<Vec<_>>();

        let blas = geometries
            .iter()
            .map(|g| Arc::new(Blas::create(device, &g)))
            .collect::<Vec<_>>();

        let instances = Arc::new(
            blas.iter()
                .map(|blas| BlasInstance {
                    blas: blas.clone(),
                    transform: vk::TransformMatrixKHR {
                        matrix: [
                            1.0, 0.0, 0.0, 0.0, //
                            0.0, 1.0, 0.0, 0.0, //
                            0.0, 0.0, 1.0, 0.0, //
                        ],
                    },
                    instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
                    instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                        0,
                        vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                    ),
                })
                .collect::<Vec<_>>(),
        );

        let tlas = Arc::new(Tlas::create(device, &instances));

        Self {
            geometries,
            blases: blas,
            tlas,
            instances,
        }
    }
}
