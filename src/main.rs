use std::ops::Deref;

use bevy_math::*;
use std::sync::Mutex;
use winit::event::DeviceEvent;
use {screen_13::prelude::*, screen_13_egui::Egui};

mod accel;
mod buffers;
#[macro_use]
mod dense_arena;
mod gbuffer;
mod glsl;
mod loaders;
mod model;
mod post;
mod render_world;
mod renderer;
mod sbt;
use gbuffer::GBuffer;
use loaders::Loader;
use model::*;
use post::*;
use renderer::*;

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let event_loop = EventLoop::new()
        .ray_tracing(true)
        .configure(|config| config.sync_display(true))
        .build()?;
    let mut cache = HashPool::new(&event_loop.device);
    let presenter = screen_13_fx::GraphicPresenter::new(&event_loop.device).unwrap();
    let lts = LinearToSrgb::new(&event_loop.device);
    let mut egui = Egui::new(&event_loop.device, event_loop.window());

    let mut rt_renderer = Mutex::new(RTRenderer::new());
    let rgen_shader = rt_renderer.lock().unwrap().insert_shader(
        Shader::new_ray_gen(
            inline_spirv::include_spirv!("src/shaders/rgen.glsl", rgen, vulkan1_2).as_slice(),
        )
        .build(),
    );
    let rchit_shader = rt_renderer.lock().unwrap().insert_shader(
        Shader::new_closest_hit(
            inline_spirv::include_spirv!("src/shaders/rchit.glsl", rchit, vulkan1_2).as_slice(),
        )
        .build(),
    );
    let miss_shader = rt_renderer.lock().unwrap().insert_shader(
        Shader::new_miss(
            inline_spirv::include_spirv!("src/shaders/rmiss.glsl", rmiss, vulkan1_2).as_slice(),
        )
        .build(),
    );
    let miss_shadow_shader = rt_renderer.lock().unwrap().insert_shader(
        Shader::new_miss(
            inline_spirv::include_spirv!("src/shaders/miss_shadow.glsl", rmiss, vulkan1_2)
                .as_slice(),
        )
        .build(),
    );
    let rgen_group = rt_renderer
        .lock()
        .unwrap()
        .insert_shader_group(ShaderGroup::General {
            general: rgen_shader,
        });
    let hit_group = rt_renderer
        .lock()
        .unwrap()
        .insert_shader_group(ShaderGroup::Triangle {
            closest_hit: rchit_shader,
            any_hit: None,
        });
    let miss_group = rt_renderer
        .lock()
        .unwrap()
        .insert_shader_group(ShaderGroup::General {
            general: miss_shader,
        });
    let miss_shadow_group = rt_renderer
        .lock()
        .unwrap()
        .insert_shader_group(ShaderGroup::General {
            general: miss_shadow_shader,
        });
    rt_renderer
        .lock()
        .unwrap()
        .set_miss_groups(vec![miss_group, miss_shadow_group]);
    rt_renderer.lock().unwrap().set_rgen_group(rgen_group);
    let loader = loaders::GltfLoader::default();
    loader.load_to(
        "./src/res/cube_scene.gltf",
        &event_loop.device,
        &rt_renderer,
        vec![hit_group],
    );
    //gpu_scene.upload_data(&event_loop.device);

    let gbuffer = GBuffer::new(&event_loop.device, [1000, 1000]);

    let mut fc = 0;
    let mut angle: f32 = std::f32::consts::PI;
    let mut camera = rt_renderer.lock().unwrap().get_camera();

    event_loop.run(|mut frame| {
        if false {
            rt_renderer
                .lock()
                .unwrap()
                .blases
                .iter()
                .for_each(|(key, _)| {
                    rt_renderer.lock().unwrap().emit(Signal::MeshChanged(*key));
                });
            rt_renderer.lock().unwrap().emit(Signal::TlasRecreated);
        }
        rt_renderer.lock().unwrap().recreate_stage(frame.device);
        rt_renderer
            .lock()
            .unwrap()
            .build_stage(&mut cache, &mut frame.render_graph);
        rt_renderer.lock().unwrap().cleanup_stage();

        rt_renderer
            .lock()
            .unwrap()
            .render(&gbuffer, &mut cache, &mut frame.render_graph);

        let color_image_node = frame.render_graph.bind_node(&gbuffer.color);

        let tmp_image_node = frame.render_graph.bind_node(
            cache
                .lease(ImageInfo::new_2d(
                    vk::Format::R8G8B8A8_UNORM,
                    1000,
                    1000,
                    vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::TRANSFER_DST
                        | vk::ImageUsageFlags::COLOR_ATTACHMENT
                        | vk::ImageUsageFlags::SAMPLED,
                ))
                .unwrap(),
        );
        lts.exec(frame.render_graph, color_image_node, tmp_image_node);

        presenter.present_image(frame.render_graph, tmp_image_node, frame.swapchain_image);

        let mut camera_changed = false;
        egui.run(
            frame.window,
            frame.events,
            frame.swapchain_image,
            &mut frame.render_graph,
            |ctx| {
                egui::Window::new("Test").show(&ctx, |ui| {
                    camera_changed |= ui
                        .add(egui::Slider::new(&mut camera.pos[0], -10.0..=10.))
                        .changed();
                    camera_changed |= ui
                        .add(egui::Slider::new(&mut camera.pos[1], -10.0..=10.))
                        .changed();
                    camera_changed |= ui
                        .add(egui::Slider::new(&mut camera.pos[2], -10.0..=10.))
                        .changed();
                    camera_changed |= ui
                        .add(egui::Slider::new(&mut angle, -10.0..=10.0))
                        .changed();
                    camera_changed |= ui
                        .add(egui::Slider::new(&mut camera.depth, 0..=16))
                        .changed();
                    if ui.button("Add instance").clicked() {
                        let mut inst = rt_renderer
                            .lock()
                            .unwrap()
                            .world
                            .instances
                            .values()
                            .next()
                            .unwrap()
                            .deref()
                            .clone();
                        inst.transform = Mat4::IDENTITY;
                        rt_renderer.lock().unwrap().insert_instance(inst);
                    }
                });
            },
        );
        let delta = frame.events.iter().find_map(|e| match e {
            Event::DeviceEvent { device_id, event } => match event {
                DeviceEvent::MouseMotion { delta } => Some(delta),
                _ => None,
            },
            _ => None,
        });
        if let Some(delta) = delta {
            let mut up = Vec4::from(camera.up).xyz();
            let quat = Quat::from_axis_angle(vec3(0., 1., 0.), delta.0 as f32 / 1000.);
            up = quat * up;
            camera.up = [up.x, up.y, up.z, 1.];
            rt_renderer.lock().unwrap().set_camera(camera);
        }

        fc += 1;
        //frame.exit();
    })?;

    Ok(())
}
