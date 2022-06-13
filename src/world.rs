use bevy_ecs::prelude::*;
use bytemuck::cast_slice;
use screen_13::prelude::*;
use std::sync::Arc;
use std::{io::BufReader, mem::size_of};
use tobj::*;

pub struct InstanceBuffer(Buffer);

/*
pub struct AccelerationStructureInfo{
    pub transform: [f32; 12],
    pub instance_custom_index: u32,
}
*/

impl InstanceBuffer {
    pub fn create(
        device: &Arc<Device>,
        instances: &[vk::AccelerationStructureInstanceKHR],
    ) -> Self {
        let buf_size = instances.len() * size_of::<vk::AccelerationStructureInstanceKHR>();
        let mut buf = Buffer::create(
            device,
            BufferInfo::new_mappable(
                (size_of::<vk::AccelerationStructureInstanceKHR>() * instances.len()) as _,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            ),
        )
        .unwrap();
        Buffer::copy_from_slice(&mut buf, 0, unsafe {
            std::slice::from_raw_parts(instances as *const _ as *const _, buf_size as _)
        });
        Self(buf)
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    pub ambient: [f32; 4],
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    pub emmision: [f32; 4],
}

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

    pub fn create_blas(
        &self,
        device: &Arc<Device>,
        rgraph: &mut RenderGraph,
        cache: &mut HashPool,
    ) {
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

pub struct RenderWorld(World);

impl RenderWorld {
    pub fn new() -> Self {
        Self(World::new())
    }
    pub fn load_gpu(&mut self, device: &Arc<Device>) {
        let (models, ..) = load_obj_buf(
            &mut BufReader::new(include_bytes!("res/onecube_scene.obj").as_slice()),
            &GPU_LOAD_OPTIONS,
            |_| {
                load_mtl_buf(&mut BufReader::new(
                    include_bytes!("res/onecube_scene.mtl").as_slice(),
                ))
            },
        )
        .unwrap();
        models
            .into_iter()
            .map(|m| GpuGeometry::from_tobj(device, m.mesh))
            .for_each(|g| {
                self.0.spawn().insert(g);
            });
    }

    pub fn create_tlas(
        &mut self,
        device: &Arc<Device>,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) {
        let instances = self
            .0
            .query::<&GpuGeometry>()
            .iter(&self.0)
            .map(|g| g.instance())
            .collect::<Vec<_>>();

        let instance_buf = Arc::new({
            let buf_size = instances.len() * size_of::<vk::AccelerationStructureInstanceKHR>();
            let data = unsafe {
                std::slice::from_raw_parts(&instances as *const _ as *const _, buf_size as _)
            };
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(
                    buf_size as _,
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                ),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });

        let tlas_geo = AccelerationStructureGeometry {
            max_primitive_count: instances.len() as _,
            flags: vk::GeometryFlagsKHR::OPAQUE,
            geometry: AccelerationStructureGeometryData::Instances {
                array_of_pointers: false,
                data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(&instance_buf)),
            },
        };
        let tlas_geo_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![tlas_geo],
        };
        let tlas_size = AccelerationStructure::size_of(device, &tlas_geo_info);
        let tlas = Arc::new(
            AccelerationStructure::create(
                device,
                AccelerationStructureInfo {
                    ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
                    size: tlas_size.create_size,
                },
            )
            .unwrap(),
        );

        {
            let instance_node = rgraph.bind_node(&instance_buf);
            let tlas_node = rgraph.bind_node(&tlas);
            let geo_nodes = self
                .0
                .query::<&GpuGeometry>()
                .iter(&self.0)
                .map(|g| rgraph.bind_node(&g.blas))
                .collect::<Vec<_>>();
            let scratch_buf = rgraph.bind_node(
                cache
                    .lease(BufferInfo::new(
                        tlas_size.build_size,
                        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                            | vk::BufferUsageFlags::STORAGE_BUFFER,
                    ))
                    .unwrap(),
            );
            let primitive_count = geo_nodes.len() as _;

            let mut pass = rgraph.begin_pass("Build TLAS").read_node(instance_node);
            for blas_node in geo_nodes {
                pass = pass.read_node(blas_node);
            }

            pass.write_node(scratch_buf)
                .write_node(tlas_node)
                .record_acceleration(move |accel| {
                    accel.build_structure(
                        tlas_node,
                        scratch_buf,
                        tlas_geo_info,
                        &[vk::AccelerationStructureBuildRangeInfoKHR {
                            first_vertex: 0,
                            primitive_count,
                            primitive_offset: 0,
                            transform_offset: 0,
                        }],
                    )
                });
        }
    }
}

pub struct GpuWorld {
    pub meshes: Vec<GpuGeometry>,
    pub materials: Arc<Buffer>,
    pub tlas: Arc<AccelerationStructure>,
}

impl GpuWorld {
    pub fn load(device: &Arc<Device>, cache: &mut HashPool) -> Self {
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

        let meshes = models
            .into_iter()
            .map(|m| GpuGeometry::from_tobj(device, m.mesh))
            .collect::<Vec<_>>();

        for geometry in meshes.iter() {
            geometry.create_blas(device, &mut rgraph, cache);
        }

        let materials = materials
            .unwrap()
            .into_iter()
            .map(|m| Material {
                ambient: [m.ambient[0], m.ambient[1], m.ambient[2], 0.],
                diffuse: [m.diffuse[0], m.diffuse[1], m.diffuse[2], 0.],
                specular: [m.specular[0], m.specular[1], m.specular[2], 0.],
                emmision: [0., 0., 0., 0.],
            })
            .collect::<Vec<_>>();
        let mat_buf = Arc::new({
            let data = cast_slice(&materials);
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(data.len() as _, vk::BufferUsageFlags::STORAGE_BUFFER),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });

        let instances = meshes.iter().map(|m| m.instance()).collect::<Vec<_>>();

        let instance_buf = Arc::new(InstanceBuffer::create(device, &instances).0);

        let tlas_geometry = AccelerationStructureGeometry {
            max_primitive_count: instances.len() as _,
            flags: vk::GeometryFlagsKHR::OPAQUE,
            geometry: AccelerationStructureGeometryData::Instances {
                array_of_pointers: false,
                data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(&instance_buf)),
            },
        };

        let tlas_geometry_info = AccelerationStructureGeometryInfo {
            ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
            geometries: vec![tlas_geometry],
        };
        let tlas_size = AccelerationStructure::size_of(device, &tlas_geometry_info);
        let tlas = Arc::new(
            AccelerationStructure::create(
                device,
                AccelerationStructureInfo {
                    ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
                    size: tlas_size.create_size,
                },
            )
            .unwrap(),
        );

        // Build TLAS
        {
            let scratch_buf = rgraph.bind_node(
                cache
                    .lease(BufferInfo::new(
                        tlas_size.build_size,
                        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                            | vk::BufferUsageFlags::STORAGE_BUFFER,
                    ))
                    .unwrap(),
            );
            //let instance_node = rgraph.bind_node(&instances);
            let tlas_node = rgraph.bind_node(&tlas);
            let instance_node = rgraph.bind_node(&instance_buf);
            let blas_nodes = meshes
                .iter()
                .map(|m| rgraph.bind_node(&m.blas))
                .collect::<Vec<_>>();
            let primitive_count = blas_nodes.len() as _;

            let mut pass = rgraph.begin_pass("Build TLAS").read_node(instance_node);
            for blas_node in blas_nodes {
                pass = pass.read_node(blas_node);
            }
            trace!("TLAS: primitive_count. {}", primitive_count);
            pass.write_node(scratch_buf)
                .write_node(tlas_node)
                .record_acceleration(move |accel| {
                    accel.build_structure(
                        tlas_node,
                        scratch_buf,
                        tlas_geometry_info,
                        &[vk::AccelerationStructureBuildRangeInfoKHR {
                            first_vertex: 0,
                            primitive_count,
                            primitive_offset: 0,
                            transform_offset: 0,
                        }],
                    )
                });
        }

        rgraph.resolve().submit(cache).unwrap();

        Self {
            meshes,
            materials: mat_buf,
            tlas,
        }
    }
}
