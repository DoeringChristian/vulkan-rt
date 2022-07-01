use std::ops::Deref;

use bevy_ecs::prelude::*;
use bevy_math::*;
use {
    bytemuck::cast_slice,
    screen_13::prelude::*,
    screen_13_egui::Egui,
    std::{io::BufReader, mem::size_of, sync::Arc},
    tobj::{load_mtl_buf, load_obj_buf, GPU_LOAD_OPTIONS},
};

mod accel;
mod buffers;
#[macro_use]
mod dense_arena;
mod model;
mod post;
mod sbt;
mod world;
use accel::*;
use bevy_transform::prelude::Transform;
use buffers::*;
use model::*;
use post::*;
use sbt::*;
use world::*;

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

    let mut rt_renderer = RTRenderer::new();
    let rgen_shader = rt_renderer.insert_shader(
        Shader::new_ray_gen(
            inline_spirv::include_spirv!("src/shaders/rgen.glsl", rgen, vulkan1_2).as_slice(),
        )
        .build(),
    );
    let rchit_shader = rt_renderer.insert_shader(
        Shader::new_closest_hit(
            inline_spirv::include_spirv!("src/shaders/rchit.glsl", rchit, vulkan1_2).as_slice(),
        )
        .build(),
    );
    let miss_shader = rt_renderer.insert_shader(
        Shader::new_miss(
            inline_spirv::include_spirv!("src/shaders/rmiss.glsl", rmiss, vulkan1_2).as_slice(),
        )
        .build(),
    );
    let rgen_group = rt_renderer.insert_shader_group(ShaderGroup::General {
        general: rgen_shader,
    });
    let hit_group = rt_renderer.insert_shader_group(ShaderGroup::Triangle {
        closest_hit: rchit_shader,
        any_hit: None,
    });
    let miss_group = rt_renderer.insert_shader_group(ShaderGroup::General {
        general: miss_shader,
    });
    rt_renderer.set_miss_groups(vec![miss_group]);
    rt_renderer.set_rgen_group(rgen_group);
    rt_renderer.append_gltf(&event_loop.device, vec![hit_group]);
    //gpu_scene.upload_data(&event_loop.device);

    let img = Arc::new(
        Image::create(
            &event_loop.device,
            ImageInfo::new_2d(
                //vk::Format::R8G8B8A8_UNORM,
                vk::Format::R32G32B32A32_SFLOAT,
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
    let mut angle: f32 = 0.;

    event_loop.run(|mut frame| {
        //gpu_scene.update_stage(frame.device);
        if fc == 3 {
            rt_renderer.instances.values_mut().next().unwrap().status = ResourceStatus::Recreated;
            for blas in rt_renderer.blases.values_mut() {
                blas.status = ResourceStatus::Recreated
            }
            //gpu_scene.insert_instance(inst);
        }
        //gpu_scene.build_accels(&mut cache, &mut frame.render_graph);
        rt_renderer.recreate_stage(frame.device);
        rt_renderer.build_stage(&mut cache, &mut frame.render_graph);
        rt_renderer.cleanup_stage();

        let image_node = frame.render_graph.bind_node(&img);

        rt_renderer.render(image_node, &mut cache, &mut frame.render_graph);

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
        lts.exec(frame.render_graph, image_node, tmp_image_node);

        presenter.present_image(frame.render_graph, tmp_image_node, frame.swapchain_image);

        let mut recreate_frame = false;
        egui.run(
            frame.window,
            frame.events,
            frame.swapchain_image,
            &mut frame.render_graph,
            |ctx| {
                egui::Window::new("Test").show(&ctx, |ui| {
                    recreate_frame |= ui
                        .add(egui::Slider::new(
                            &mut rt_renderer.camera.pos[0],
                            -10.0..=10.,
                        ))
                        .changed();
                    recreate_frame |= ui
                        .add(egui::Slider::new(
                            &mut rt_renderer.camera.pos[1],
                            -10.0..=10.,
                        ))
                        .changed();
                    recreate_frame |= ui
                        .add(egui::Slider::new(
                            &mut rt_renderer.camera.pos[2],
                            -10.0..=10.,
                        ))
                        .changed();
                    recreate_frame |= ui
                        .add(egui::Slider::new(&mut angle, -10.0..=10.0))
                        .changed();
                    recreate_frame |= ui
                        .add(egui::Slider::new(&mut rt_renderer.camera.depth, 0..=16))
                        .changed();
                    if ui.button("Add instance").clicked() {
                        let mut inst = rt_renderer
                            .instances
                            .values()
                            .next()
                            .unwrap()
                            .deref()
                            .clone();
                        inst.transform = Transform::from_xyz(0., 0., 0.);
                        rt_renderer.insert_instance(inst);
                    }
                });
            },
        );
        if recreate_frame {
            let v = Vec3::new(angle.sin(), 0., angle.cos());
            rt_renderer.camera.up = [v.x, v.y, v.z, 1.];
            //println!("{:#?}", gpu_scene.camera.right);
            rt_renderer.camera.fc = 0;
        }

        fc += 1;
        //frame.exit();
    })?;

    Ok(())
}
