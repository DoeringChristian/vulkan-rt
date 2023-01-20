mod accel;
mod array;
mod glsl;
mod loaders;
mod post;
mod renderer;
mod sbt;
mod scene;

use screen_13::prelude::*;

use self::loaders::Loader;
use self::scene::Scene;

fn main() {
    pretty_env_logger::init();
    let sc13 = EventLoop::new()
        .debug(true)
        .ray_tracing(true)
        .build()
        .unwrap();
    let device = &sc13.device;
    let mut cache = HashPool::new(device);

    let presenter = screen_13_fx::GraphicPresenter::new(device).unwrap();
    let pt_renderer = renderer::PTRenderer::create(device);

    let mut scene = Scene::default();
    let loader = loaders::GltfLoader::default();
    loader.append("assets/cornell-box.gltf", &mut scene);

    let mut i = 0;

    sc13.run(|frame| {
        if i == 0 {
            scene.update(frame.device, &mut cache, frame.render_graph);
            println!("{:#?}", scene.cameras);
        }
        frame.render_graph.clear_color_image(frame.swapchain_image);
        i += 1;
    })
    .unwrap();

    // pretty_env_logger::init();
    //
    // let event_loop = EventLoop::new()
    //     .ray_tracing(true)
    //     .configure(|config| config.sync_display(true))
    //     .build()?;
    // let mut cache = HashPool::new(&event_loop.device);
    // let presenter = screen_13_fx::GraphicPresenter::new(&event_loop.device).unwrap();
    // let lts = LinearToSrgb::new(&event_loop.device);
    //
    // let mut rt_renderer = Arc::new(Mutex::new(RTRenderer::new()));
    // let rgen_shader = rt_renderer.lock().unwrap().insert_shader(
    //     Shader::new_ray_gen(
    //         inline_spirv::include_spirv!("src/shaders/rgen.glsl", rgen, vulkan1_2).as_slice(),
    //     )
    //     .build(),
    // );
    // let rchit_shader = rt_renderer.lock().unwrap().insert_shader(
    //     Shader::new_closest_hit(
    //         inline_spirv::include_spirv!("src/shaders/rchit.glsl", rchit, vulkan1_2).as_slice(),
    //     )
    //     .build(),
    // );
    // let miss_shader = rt_renderer.lock().unwrap().insert_shader(
    //     Shader::new_miss(
    //         inline_spirv::include_spirv!("src/shaders/rmiss.glsl", rmiss, vulkan1_2).as_slice(),
    //     )
    //     .build(),
    // );
    // let miss_shadow_shader = rt_renderer.lock().unwrap().insert_shader(
    //     Shader::new_miss(
    //         inline_spirv::include_spirv!("src/shaders/miss_shadow.glsl", rmiss, vulkan1_2)
    //             .as_slice(),
    //     )
    //     .build(),
    // );
    // let rgen_group = rt_renderer
    //     .lock()
    //     .unwrap()
    //     .insert_shader_group(ShaderGroup::General {
    //         general: rgen_shader,
    //     });
    // let hit_group = rt_renderer
    //     .lock()
    //     .unwrap()
    //     .insert_shader_group(ShaderGroup::Triangle {
    //         closest_hit: rchit_shader,
    //         any_hit: None,
    //     });
    // let miss_group = rt_renderer
    //     .lock()
    //     .unwrap()
    //     .insert_shader_group(ShaderGroup::General {
    //         general: miss_shader,
    //     });
    // let miss_shadow_group = rt_renderer
    //     .lock()
    //     .unwrap()
    //     .insert_shader_group(ShaderGroup::General {
    //         general: miss_shadow_shader,
    //     });
    // rt_renderer
    //     .lock()
    //     .unwrap()
    //     .set_miss_groups(vec![miss_group, miss_shadow_group]);
    // rt_renderer.lock().unwrap().set_rgen_group(rgen_group);
    // let loader = loaders::GltfLoader::default();
    // let device = event_loop.device.clone();
    // let loader_dst = rt_renderer.clone();
    // std::thread::spawn(move || {
    //     loader.load_to(
    //         "./src/res/cube_scene.gltf",
    //         &device,
    //         &loader_dst,
    //         vec![hit_group],
    //     );
    // });
    //
    // let gbuffer = GBuffer::new(&event_loop.device, [1000, 1000]);
    //
    // let mut fc = 0;
    // let mut angle: f32 = std::f32::consts::PI;
    //
    // event_loop.run(|mut frame| {
    //     if false {
    //         rt_renderer
    //             .lock()
    //             .unwrap()
    //             .blases
    //             .iter()
    //             .for_each(|(key, _)| {
    //                 rt_renderer.lock().unwrap().emit(Signal::MeshChanged(*key));
    //             });
    //         rt_renderer.lock().unwrap().emit(Signal::TlasRecreated);
    //     }
    //     rt_renderer.lock().unwrap().recreate_stage(frame.device);
    //     rt_renderer
    //         .lock()
    //         .unwrap()
    //         .build_stage(&mut cache, &mut frame.render_graph);
    //     rt_renderer.lock().unwrap().cleanup_stage();
    //
    //     rt_renderer
    //         .lock()
    //         .unwrap()
    //         .render(&gbuffer, &mut cache, &mut frame.render_graph);
    //
    //     let color_image_node = frame.render_graph.bind_node(&gbuffer.color);
    //
    //     let tmp_image_node = frame.render_graph.bind_node(
    //         cache
    //             .lease(ImageInfo::new_2d(
    //                 vk::Format::R8G8B8A8_UNORM,
    //                 1000,
    //                 1000,
    //                 vk::ImageUsageFlags::TRANSFER_SRC
    //                     | vk::ImageUsageFlags::TRANSFER_DST
    //                     | vk::ImageUsageFlags::COLOR_ATTACHMENT
    //                     | vk::ImageUsageFlags::SAMPLED,
    //             ))
    //             .unwrap(),
    //     );
    //     lts.exec(frame.render_graph, color_image_node, tmp_image_node);
    //
    //     fc += 1;
    //     //frame.exit();
    // })?;
    //
    // Ok(())
}
