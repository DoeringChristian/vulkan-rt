use {
    bytemuck::cast_slice,
    screen_13::prelude::*,
    std::{io::BufReader, mem::size_of, sync::Arc},
    tobj::{load_mtl_buf, load_obj_buf, GPU_LOAD_OPTIONS},
};

mod sbt;
use sbt::*;
mod world;
use world::*;

fn align_up(val: u32, atom: u32) -> u32 {
    (val + atom - 1) & !(atom - 1)
}

fn create_ray_trace_pipeline(device: &Arc<Device>) -> Result<Arc<RayTracePipeline>, DriverError> {
    Ok(Arc::new(RayTracePipeline::create(
        device,
        RayTracePipelineInfo::new()
            .max_ray_recursion_depth(1)
            .build(),
        [
            Shader::new_ray_gen(
                inline_spirv::include_spirv!("src/shaders/rgen.glsl", rgen, vulkan1_2).as_slice(),
            ),
            Shader::new_closest_hit(
                inline_spirv::include_spirv!("src/shaders/rchit.glsl", rchit, vulkan1_2).as_slice(),
            ),
            Shader::new_miss(
                inline_spirv::include_spirv!("src/shaders/rmiss.glsl", rmiss, vulkan1_2).as_slice(),
            ),
            Shader::new_miss(
                inline_spirv::include_spirv!("src/shaders/shadow_rmiss.glsl", rmiss, vulkan1_2)
                    .as_slice(),
            ),
        ],
        [
            RayTraceShaderGroup::new_general(0),
            RayTraceShaderGroup::new_triangles(1, None),
            RayTraceShaderGroup::new_general(2),
            RayTraceShaderGroup::new_general(3),
        ],
    )?))
}

fn load_scene_buffers(
    device: &Arc<Device>,
) -> Result<(Arc<Buffer>, Arc<Buffer>, u32, u32, Arc<Buffer>, Arc<Buffer>), DriverError> {
    use std::slice::from_raw_parts;

    let (models, materials, ..) = load_obj_buf(
        &mut BufReader::new(include_bytes!("res/cube_scene.obj").as_slice()),
        &GPU_LOAD_OPTIONS,
        |_| {
            load_mtl_buf(&mut BufReader::new(
                include_bytes!("res/cube_scene.mtl").as_slice(),
            ))
        },
    )
    .map_err(|err| {
        warn!("{err}");

        DriverError::InvalidData
    })?;
    let materials = materials.map_err(|err| {
        warn!("{err}");

        DriverError::InvalidData
    })?;

    let mut indices = vec![];
    let mut positions = vec![];
    for model in &models {
        let base_index = positions.len() as u32 / 3;
        for index in &model.mesh.indices {
            indices.push(*index + base_index);
        }

        for position in &model.mesh.positions {
            positions.push(*position);
        }
    }

    let index_buf = {
        let data = cast_slice(&indices);
        let mut buf = Buffer::create(
            device,
            BufferInfo::new_mappable(
                data.len() as _,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            ),
        )?;
        Buffer::copy_from_slice(&mut buf, 0, data);
        buf
    };

    let vertex_buf = {
        let data = cast_slice(&positions);
        let mut buf = Buffer::create(
            device,
            BufferInfo::new_mappable(
                data.len() as _,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            ),
        )?;
        Buffer::copy_from_slice(&mut buf, 0, data);
        buf
    };

    let material_id_buf = {
        let mut material_ids = vec![];
        for model in &models {
            for _ in 0..model.mesh.indices.len() / 3 {
                material_ids.push(model.mesh.material_id.unwrap() as u32);
            }
        }
        let data = cast_slice(&material_ids);
        let mut buf = Buffer::create(
            device,
            BufferInfo::new_mappable(data.len() as _, vk::BufferUsageFlags::STORAGE_BUFFER),
        )?;
        Buffer::copy_from_slice(&mut buf, 0, data);
        buf
    };

    let material_buf = {
        let materials = materials
            .iter()
            .map(|material| {
                [
                    material.ambient[0],
                    material.ambient[1],
                    material.ambient[2],
                    0.0,
                    material.diffuse[0],
                    material.diffuse[1],
                    material.diffuse[2],
                    0.0,
                    material.specular[0],
                    material.specular[1],
                    material.specular[2],
                    0.0,
                    1.0,
                    1.0,
                    1.0,
                    0.0,
                ]
            })
            .collect::<Box<[_]>>();
        let buf_len = materials.len() * 64;
        let mut buf = Buffer::create(
            device,
            BufferInfo::new_mappable(buf_len as _, vk::BufferUsageFlags::STORAGE_BUFFER),
        )?;
        Buffer::copy_from_slice(&mut buf, 0, unsafe {
            from_raw_parts(materials.as_ptr() as *const _, buf_len)
        });
        buf
    };

    Ok((
        Arc::new(index_buf),
        Arc::new(vertex_buf),
        indices.len() as u32 / 3,
        positions.len() as u32 / 3,
        Arc::new(material_id_buf),
        Arc::new(material_buf),
    ))
}

/// Adapted from http://williamlewww.com/showcase_website/vk_khr_ray_tracing_tutorial/index.html
fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let event_loop = EventLoop::new().ray_tracing(true).build()?;
    let mut cache = HashPool::new(&event_loop.device);

    // ------------------------------------------------------------------------------------------ //
    // Setup the ray tracing pipeline
    // ------------------------------------------------------------------------------------------ //
    let ray_trace_pipeline = create_ray_trace_pipeline(&event_loop.device)?;

    // Setup the Shader Binding Table
    let sbt_info = SbtBufferInfo {
        rgen_index: 0,
        hit_indices: &[1],
        miss_indices: &[2, 3],
        callable_indices: &[],
    };

    let sbt = SbtBuffer::create(&event_loop.device, sbt_info, &ray_trace_pipeline)?;

    // ------------------------------------------------------------------------------------------ //
    // Load the .obj cube scene
    // ------------------------------------------------------------------------------------------ //

    let (index_buf, vertex_buf, triangle_count, vertex_count, material_id_buf, material_buf) =
        load_scene_buffers(&event_loop.device)?;

    let world = GpuWorld::load(&event_loop.device, &mut cache);

    // ------------------------------------------------------------------------------------------ //
    // Create the bottom level acceleration structure
    // ------------------------------------------------------------------------------------------ //

    let blas_geometry_info = AccelerationStructureGeometryInfo {
        ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
        flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
        geometries: vec![AccelerationStructureGeometry {
            max_primitive_count: triangle_count,
            flags: vk::GeometryFlagsKHR::OPAQUE,
            geometry: AccelerationStructureGeometryData::Triangles {
                index_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(&index_buf)),
                index_type: vk::IndexType::UINT32,
                max_vertex: vertex_count,
                transform_data: None,
                vertex_data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(
                    &vertex_buf,
                )),
                vertex_format: vk::Format::R32G32B32_SFLOAT,
                vertex_stride: 12,
            },
        }],
    };
    let blas_size = AccelerationStructure::size_of(&event_loop.device, &blas_geometry_info);
    let blas = Arc::new(AccelerationStructure::create(
        &event_loop.device,
        AccelerationStructureInfo {
            ty: vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            size: blas_size.create_size,
        },
    )?);
    let blas_device_address = AccelerationStructure::device_address(&blas);

    // ------------------------------------------------------------------------------------------ //
    // Create an instance buffer, which is just one instance for the single BLAS
    // ------------------------------------------------------------------------------------------ //

    let instance_buf = Arc::new({
        let mut buffer = Buffer::create(
            &event_loop.device,
            BufferInfo::new_mappable(
                size_of::<vk::AccelerationStructureInstanceKHR>() as _,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            ),
        )?;
        Buffer::copy_from_slice(&mut buffer, 0, unsafe {
            std::slice::from_raw_parts(
                &vk::AccelerationStructureInstanceKHR {
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
                        device_handle: blas_device_address,
                    },
                } as *const _ as *const _,
                size_of::<vk::AccelerationStructureInstanceKHR>(),
            )
        });

        buffer
    });

    // ------------------------------------------------------------------------------------------ //
    // Create the top level acceleration structure
    // ------------------------------------------------------------------------------------------ //

    let tlas_geometry_info = AccelerationStructureGeometryInfo {
        ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
        flags: vk::BuildAccelerationStructureFlagsKHR::empty(),
        geometries: vec![AccelerationStructureGeometry {
            max_primitive_count: 1,
            flags: vk::GeometryFlagsKHR::OPAQUE,
            geometry: AccelerationStructureGeometryData::Instances {
                array_of_pointers: false,
                data: DeviceOrHostAddress::DeviceAddress(Buffer::device_address(&instance_buf)),
            },
        }],
    };
    let tlas_size = AccelerationStructure::size_of(&event_loop.device, &tlas_geometry_info);
    let tlas = Arc::new(AccelerationStructure::create(
        &event_loop.device,
        AccelerationStructureInfo {
            ty: vk::AccelerationStructureTypeKHR::TOP_LEVEL,
            size: tlas_size.create_size,
        },
    )?);

    // ------------------------------------------------------------------------------------------ //
    // Build the BLAS and TLAS; note that we don't drop the cache and so there is no CPU stall
    // ------------------------------------------------------------------------------------------ //

    {
        let mut render_graph = RenderGraph::new();
        let index_node = render_graph.bind_node(&index_buf);
        let vertex_node = render_graph.bind_node(&vertex_buf);
        let blas_node = render_graph.bind_node(&blas);

        {
            let scratch_buf = render_graph.bind_node(Buffer::create(
                &event_loop.device,
                BufferInfo::new(
                    blas_size.build_size,
                    vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ),
            )?);
            let scratch_buf = render_graph.bind_node(
                cache
                    .lease(BufferInfo::new(
                        blas_size.build_size,
                        vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                            | vk::BufferUsageFlags::STORAGE_BUFFER,
                    ))
                    .unwrap(),
            );

            render_graph
                .begin_pass("Build BLAS")
                .read_node(index_node)
                .read_node(vertex_node)
                .write_node(blas_node)
                .write_node(scratch_buf)
                .record_acceleration(move |accel| {
                    accel.build_structure(
                        blas_node,
                        scratch_buf,
                        blas_geometry_info,
                        &[vk::AccelerationStructureBuildRangeInfoKHR {
                            first_vertex: 0,
                            primitive_count: triangle_count,
                            primitive_offset: 0,
                            transform_offset: 0,
                        }],
                    )
                });
        }

        {
            let scratch_buf = render_graph.bind_node(Buffer::create(
                &event_loop.device,
                BufferInfo::new(
                    tlas_size.build_size,
                    vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ),
            )?);
            let instance_node = render_graph.bind_node(&instance_buf);
            let tlas_node = render_graph.bind_node(&tlas);

            render_graph
                .begin_pass("Build TLAS")
                .read_node(blas_node)
                .read_node(instance_node)
                .write_node(scratch_buf)
                .write_node(tlas_node)
                .record_acceleration(move |accel| {
                    accel.build_structure(
                        tlas_node,
                        scratch_buf,
                        tlas_geometry_info,
                        &[vk::AccelerationStructureBuildRangeInfoKHR {
                            first_vertex: 0,
                            primitive_count: 1,
                            primitive_offset: 0,
                            transform_offset: 0,
                        }],
                    );
                });
        }

        render_graph.resolve().submit(&mut cache)?;
    }

    // ------------------------------------------------------------------------------------------ //
    // Setup some state variables to hold between frames
    // ------------------------------------------------------------------------------------------ //

    let mut frame_count = 0;
    let mut image = None;
    let mut keyboard = KeyBuf::default();
    let mut position = [1.39176035, 3.51999736, 5.59873962, 1f32];
    let right = [0.999987483f32, 0.00000000, -0.00499906437, 1.00000000];
    let up = [0f32, 1.0, 0.0, 1.0];
    let forward = [-0.00499906437f32, 0.00000000, -0.999987483, 1.00000000];

    // The event loop consists of:
    // - Lazy-init the storage image used to accumulate light
    // - Handle input
    // - Update the camera uniform buffer
    // - Trace the image
    // - Copy image to the swapchain
    event_loop.run(|mut frame| {
        if image.is_none() {
            image = Some(Arc::new(
                cache
                    .lease(ImageInfo::new_2d(
                        frame.render_graph.node_info(frame.swapchain_image).fmt,
                        frame.width,
                        frame.height,
                        vk::ImageUsageFlags::STORAGE
                            | vk::ImageUsageFlags::TRANSFER_DST
                            | vk::ImageUsageFlags::TRANSFER_SRC,
                    ))
                    .unwrap(),
            ));
        }

        let image_node = frame.render_graph.bind_node(image.as_ref().unwrap());

        {
            update_keyboard(&mut keyboard, frame.events);

            const SPEED: f32 = 0.1f32;

            if keyboard.is_pressed(&VirtualKeyCode::Left) {
                position[0] -= SPEED;
            } else if keyboard.is_pressed(&VirtualKeyCode::Right) {
                position[0] += SPEED;
            } else if keyboard.is_pressed(&VirtualKeyCode::Up) {
                position[2] -= SPEED;
            } else if keyboard.is_pressed(&VirtualKeyCode::Down) {
                position[2] += SPEED;
            } else if keyboard.is_pressed(&VirtualKeyCode::Space) {
                position[1] -= SPEED;
            } else if keyboard.is_pressed(&VirtualKeyCode::LAlt) {
                position[1] += SPEED;
            }

            if keyboard.any_pressed() {
                frame_count = 0;
                frame.render_graph.clear_color_image(image_node);
            } else {
                frame_count += 1;
            }
        }

        let camera_buf = frame.render_graph.bind_node({
            #[repr(C)]
            struct Camera {
                position: [f32; 4],
                right: [f32; 4],
                up: [f32; 4],
                forward: [f32; 4],
                frame_count: u32,
            }

            let mut buf = cache
                .lease(BufferInfo::new_mappable(
                    size_of::<Camera>() as _,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                ))
                .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, unsafe {
                std::slice::from_raw_parts(
                    &Camera {
                        position,
                        right,
                        up,
                        forward,
                        frame_count,
                    } as *const _ as *const _,
                    size_of::<Camera>(),
                )
            });

            buf
        });
        let blas_node = frame.render_graph.bind_node(&blas);
        let tlas_node = frame.render_graph.bind_node(&tlas);
        let index_buf_node = frame.render_graph.bind_node(&index_buf);
        let vertex_buf_node = frame.render_graph.bind_node(&vertex_buf);
        let material_id_buf_node = frame.render_graph.bind_node(&material_id_buf);
        let material_buf_node = frame.render_graph.bind_node(&material_buf);
        let sbt_node = frame.render_graph.bind_node(sbt.buffer());

        let sbt_rgen = sbt.rgen();
        let sbt_miss = sbt.miss();
        let sbt_hit = sbt.hit();
        let sbt_callable = sbt.callable();

        frame
            .render_graph
            .begin_pass("basic ray tracer")
            .bind_pipeline(&ray_trace_pipeline)
            .access_node(
                blas_node,
                AccessType::RayTracingShaderReadAccelerationStructure,
            )
            .access_node(sbt_node, AccessType::RayTracingShaderReadOther)
            .access_descriptor(
                0,
                tlas_node,
                AccessType::RayTracingShaderReadAccelerationStructure,
            )
            .access_descriptor(1, camera_buf, AccessType::RayTracingShaderReadOther)
            .access_descriptor(2, index_buf_node, AccessType::RayTracingShaderReadOther)
            .access_descriptor(3, vertex_buf_node, AccessType::RayTracingShaderReadOther)
            .write_descriptor(4, image_node)
            .access_descriptor(
                5,
                material_id_buf_node,
                AccessType::RayTracingShaderReadOther,
            )
            .access_descriptor(6, material_buf_node, AccessType::RayTracingShaderReadOther)
            .record_ray_trace(move |ray_trace| {
                ray_trace.trace_rays(
                    &sbt_rgen,
                    &sbt_miss,
                    &sbt_hit,
                    &sbt_callable,
                    frame.width,
                    frame.height,
                    1,
                );
            })
            .submit_pass()
            .copy_image(image_node, frame.swapchain_image);
    })?;

    Ok(())
}
