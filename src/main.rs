mod accel;
mod array;
mod glsl;
mod loaders;
mod post;
mod renderer;
mod sbt;
mod scene;

use screen_13::prelude::*;
use std::sync::Arc;

use self::loaders::Loader;
use self::post::Denoiser;
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
    let denoiser = Denoiser::new(device, 1000, 1000);

    let mut scene = Scene::default();
    let loader = loaders::GltfLoader::default();
    loader.append("assets/cornell-box.gltf", &mut scene);

    let mut i = 0;

    sc13.run(|frame| {
        if i == 0 {
            scene.update(frame.device, &mut cache, frame.render_graph);
            println!("{}", scene.material_data.as_ref().unwrap().count());
        }
        let img = cache
            .lease(ImageInfo::new_2d(
                vk::Format::R32G32B32A32_SFLOAT,
                1000,
                1000,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            ))
            .unwrap();
        let img = frame.render_graph.bind_node(img);

        pt_renderer.bind_and_render(
            &scene,
            img,
            i,
            1024,
            1024,
            0,
            &mut cache,
            frame.render_graph,
        );

        let denoised = denoiser.denoise(img, i, frame.render_graph);

        presenter.present_image(frame.render_graph, denoised, frame.swapchain_image);
        //frame.render_graph.clear_color_image(frame.swapchain_image);
        i += 1;
    })
    .unwrap();
}
