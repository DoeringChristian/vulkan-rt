use bevy_ecs::prelude::*;
use {
    bytemuck::cast_slice,
    screen_13::prelude::*,
    std::{io::BufReader, mem::size_of, sync::Arc},
    tobj::{load_mtl_buf, load_obj_buf, GPU_LOAD_OPTIONS},
};

mod accel;
mod buffers;
mod sbt;
mod world;
use accel::*;
use buffers::*;
use sbt::*;
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
        ],
        [
            RayTraceShaderGroup::new_general(0),
            RayTraceShaderGroup::new_triangles(1, None),
            RayTraceShaderGroup::new_general(2),
        ],
    )?))
}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let event_loop = EventLoop::new().ray_tracing(true).build()?;
    let mut cache = HashPool::new(&event_loop.device);
    let presenter = screen_13_fx::GraphicPresenter::new(&event_loop.device).unwrap();

    // ------------------------------------------------------------------------------------------ //
    // Setup the ray tracing pipeline
    // ------------------------------------------------------------------------------------------ //
    let ray_trace_pipeline = create_ray_trace_pipeline(&event_loop.device)?;

    // Setup the Shader Binding Table
    let sbt_info = SbtBufferInfo {
        rgen_index: 0,
        hit_indices: &[1],
        miss_indices: &[2],
        callable_indices: &[],
    };

    let sbt = SbtBuffer::create(&event_loop.device, sbt_info, &ray_trace_pipeline)?;

    // ------------------------------------------------------------------------------------------ //
    // Load the .obj cube scene
    // ------------------------------------------------------------------------------------------ //

    let mut scene = Scene::new();
    scene.load(&event_loop.device);

    let img = Arc::new(
        Image::create(
            &event_loop.device,
            ImageInfo::new_2d(
                vk::Format::R8G8B8A8_UNORM,
                100,
                100,
                vk::ImageUsageFlags::STORAGE
                    | vk::ImageUsageFlags::TRANSFER_SRC
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::SAMPLED,
            ),
        )
        .unwrap(),
    );

    let mut fc = 0;

    event_loop.run(|mut frame| {
        if fc == 0 {
            scene.build_accels(&mut cache, &mut frame.render_graph);
        } else {
            //world.instances[0].transform.matrix[3] += 0.01;
            //world.update_tlas(frame.device, &mut cache, &mut frame.render_graph);
        }

        let image_node = frame.render_graph.bind_node(&img);

        let blas_nodes = scene
            .blases
            .iter()
            .map(|b| frame.render_graph.bind_node(&b.accel))
            .collect::<Vec<_>>();
        let material_node = frame
            .render_graph
            .bind_node(&scene.tlas.as_ref().unwrap().material_buf.data);
        let tlas_node = frame
            .render_graph
            .bind_node(&scene.tlas.as_ref().unwrap().accel);
        let sbt_node = frame.render_graph.bind_node(sbt.buffer());

        let sbt_rgen = sbt.rgen();
        let sbt_miss = sbt.miss();
        let sbt_hit = sbt.hit();
        let sbt_callable = sbt.callable();

        trace!("blas_count: {}", blas_nodes.len());

        let mut pass: PipelinePassRef<RayTracePipeline> = frame
            .render_graph
            .begin_pass("basic ray tracer")
            .bind_pipeline(&ray_trace_pipeline);
        for blas_node in blas_nodes {
            pass = pass.access_node(
                blas_node,
                AccessType::RayTracingShaderReadAccelerationStructure,
            );
        }
        pass.access_node(sbt_node, AccessType::RayTracingShaderReadOther)
            .access_descriptor(
                0,
                tlas_node,
                AccessType::RayTracingShaderReadAccelerationStructure,
            )
            .write_descriptor(1, image_node)
            .read_descriptor(2, material_node)
            .record_ray_trace(move |ray_trace| {
                ray_trace.trace_rays(&sbt_rgen, &sbt_miss, &sbt_hit, &sbt_callable, 100, 100, 1);
            });
        presenter.present_image(frame.render_graph, image_node, frame.swapchain_image);
        fc += 1;
        //frame.exit();
    })?;

    Ok(())
}
