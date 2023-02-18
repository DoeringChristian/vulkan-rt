mod accel;
mod array;
mod common;
mod loaders;
mod post;
mod renderer;
mod sbt;
mod scene;

use crevice::std140::AsStd140;
use screen_13::prelude::*;
use std::sync::Arc;
use winit::event::DeviceEvent;

use self::loaders::Loader;
use self::post::{Denoiser, LinearToSrgb};
use self::renderer::{PTRenderer, RestirRenderer};
use self::scene::Scene;
use glam::*;

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
    let mut pt_renderer = PTRenderer::new(device);
    let denoiser = Denoiser::new(device, 1024, 1024);
    let linear_to_srgb = LinearToSrgb::new(device);

    let mut scene = Scene::default();
    let loader = loaders::GltfLoader::default();
    loader.append("assets/cornell-box.gltf", &mut scene);

    let mut i = 0;
    let mut pitch = 0.;
    let mut yaw = 0.;

    sc13.run(|frame| {
        if i == 0 {
            scene.update(frame.device, &mut cache, frame.render_graph);
            println!("{}", scene.material_data.as_ref().unwrap().count());
        }

        // for event in frame.events {
        //     match event {
        //         Event::DeviceEvent { device_id, event } => match event {
        //             DeviceEvent::MouseMotion { delta } => {
        //                 pitch += delta.0 as f32 * 0.01;
        //                 yaw += delta.1 as f32 * 0.01;
        //                 scene.cameras[0].to_world = scene.cameras[0].to_world
        //                     * glam::Mat4::from_rotation_y(delta.0 as f32 * 0.01)
        //                     * glam::Mat4::from_rotation_x(-delta.1 as f32 * 0.01);
        //                 scene.update_camera(frame.device, &mut cache, frame.render_graph);
        //             }
        //             _ => {}
        //         },
        //         _ => {}
        //     }
        // }

        let scene = scene.bind(frame.render_graph);

        let gbuffer =
            pt_renderer.bind_and_render(&scene, i, 1024, 1024, 0, &mut cache, frame.render_graph);

        let denoised = denoiser.denoise(gbuffer.color, i, frame.render_graph);

        let img_srgb = linear_to_srgb.record(denoised, &mut cache, frame.render_graph);

        presenter.present_image(frame.render_graph, img_srgb, frame.swapchain_image);
        //frame.render_graph.clear_color_image(frame.swapchain_image);
        i += 1;
    })
    .unwrap();
}
