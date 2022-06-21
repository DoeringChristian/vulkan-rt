use bevy_ecs::prelude::*;
use {
    bytemuck::cast_slice,
    screen_13::prelude::*,
    std::{io::BufReader, mem::size_of, sync::Arc},
    tobj::{load_mtl_buf, load_obj_buf, GPU_LOAD_OPTIONS},
};

mod accel;
mod buffers;
mod model;
mod sbt;
mod world;
use accel::*;
use buffers::*;
use model::*;
use sbt::*;
use world::*;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstant {
    frame_count: u32,
}

fn create_ray_trace_pipeline(device: &Arc<Device>) -> Result<Arc<RayTracePipeline>, DriverError> {
    Ok(Arc::new(RayTracePipeline::create(
        device,
        RayTracePipelineInfo::new()
            .max_ray_recursion_depth(3)
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
    scene.load_gltf(&event_loop.device);
    let gpu_scene = GpuScene::create(&event_loop.device, &mut scene);

    let img = Arc::new(
        Image::create(
            &event_loop.device,
            ImageInfo::new_2d(
                vk::Format::R8G8B8A8_UNORM,
                1000,
                1000,
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
        if fc < 2 {
            // The heck... why do we need to create it twice if binding bindless with two
            // instance?
            gpu_scene.build_accels(&mut cache, &mut frame.render_graph);
        } else {
            //world.instances[0].transform.matrix[3] += 0.01;
            //world.update_tlas(frame.device, &mut cache, &mut frame.render_graph);
        }

        let image_node = frame.render_graph.bind_node(&img);

        let blas_nodes = gpu_scene
            .blases
            .iter()
            .map(|b| frame.render_graph.bind_node(&b.accel))
            .collect::<Vec<_>>();
        let material_node = frame
            .render_graph
            .bind_node(&gpu_scene.tlas.material_buf.data);
        let instancedata_nodes = frame
            .render_graph
            .bind_node(&gpu_scene.tlas.instancedata_buf.data);
        let tlas_node = frame.render_graph.bind_node(&gpu_scene.tlas.accel);
        let sbt_node = frame.render_graph.bind_node(sbt.buffer());
        let index_nodes = gpu_scene
            .blases
            .iter()
            .map(|b| frame.render_graph.bind_node(&b.geometry.indices.data))
            .collect::<Vec<_>>();
        let position_nodes = gpu_scene
            .blases
            .iter()
            .map(|b| frame.render_graph.bind_node(&b.geometry.positions.data))
            .collect::<Vec<_>>();

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
        pass = pass
            .access_node(sbt_node, AccessType::RayTracingShaderReadOther)
            .access_descriptor(
                (0, 0),
                tlas_node,
                AccessType::RayTracingShaderReadAccelerationStructure,
            )
            .write_descriptor((0, 1), image_node)
            .read_descriptor((0, 2), instancedata_nodes)
            .read_descriptor((0, 3), material_node);

        //pass = pass.read_descriptor((0, 4, [0]), index_nodes[0]);
        for (i, node) in index_nodes.iter().enumerate() {
            pass = pass.read_descriptor((0, 4, [i as _]), *node);
        }
        for (i, node) in position_nodes.iter().enumerate() {
            pass = pass.read_descriptor((0, 5, [i as _]), *node);
        }
        let push_constant = PushConstant {
            frame_count: fc as _,
        };
        trace!("fc: {}", fc);
        pass.record_ray_trace(move |ray_trace| {
            ray_trace.push_constants(cast_slice(&[push_constant]));
            ray_trace.trace_rays(&sbt_rgen, &sbt_miss, &sbt_hit, &sbt_callable, 1000, 1000, 2);
        });
        presenter.present_image(frame.render_graph, image_node, frame.swapchain_image);
        fc += 1;
        //frame.exit();
    })?;

    Ok(())
}
