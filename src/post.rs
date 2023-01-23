use crevice::std140::{AsStd140, Std140};
use inline_spirv::include_spirv;
use screen_13::prelude::*;
use std::sync::Arc;

pub struct Denoiser {
    ppl: Arc<ComputePipeline>,
    avg: Arc<Image>,
}
impl Denoiser {
    pub fn new(device: &Arc<Device>, width: u32, height: u32) -> Self {
        Self {
            avg: Arc::new(
                Image::create(
                    device,
                    ImageInfo::new_2d(
                        vk::Format::R32G32B32A32_SFLOAT,
                        width,
                        height,
                        vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                    ),
                )
                .unwrap(),
            ),
            ppl: Arc::new(
                ComputePipeline::create(
                    device,
                    ComputePipelineInfo::default(),
                    Shader::new_compute(
                        include_spirv!("src/shaders/util/denoiser.glsl", comp).as_slice(),
                    ),
                )
                .unwrap(),
            ),
        }
    }
    pub fn denoise(
        &self,
        current: impl Into<AnyImageNode>,
        frame_count: u32,
        rgraph: &mut RenderGraph,
    ) -> ImageNode {
        let avg = rgraph.bind_node(&self.avg);
        let current = current.into();
        let avg_info = rgraph.node_info(avg);
        let current_info = rgraph.node_info(current);

        assert!(avg_info.width == current_info.width);
        assert!(avg_info.height == current_info.height);

        let width = avg_info.width;
        let height = avg_info.height;

        #[derive(AsStd140)]
        struct PushConstant {
            frame_count: u32,
        }

        let push_constant = PushConstant { frame_count };

        rgraph
            .begin_pass("Denoiser")
            .bind_pipeline(&self.ppl)
            .read_descriptor((0, 0), current)
            .write_descriptor((0, 1), avg)
            .record_compute(move |compute, _| {
                compute.push_constants(push_constant.as_std140().as_bytes());
                compute.dispatch(width, height, 1);
            });
        avg
    }
}

pub struct LinearToSrgb {
    ppl: Arc<GraphicPipeline>,
}

impl LinearToSrgb {
    pub fn new(device: &Arc<Device>) -> Self {
        Self {
            ppl: Arc::new(
                GraphicPipeline::create(
                    device,
                    GraphicPipelineInfo::new(),
                    [
                        Shader::new_vertex(
                            include_spirv!("./src/shaders/util/quad_vert.glsl", vert).as_slice(),
                        ),
                        Shader::new_fragment(
                            include_spirv!("./src/shaders/util/linear_to_srgb_frag.glsl", frag)
                                .as_slice(),
                        ),
                    ],
                )
                .unwrap(),
            ),
        }
    }
    pub fn record(
        &self,
        src: impl Into<AnyImageNode>,
        dst: impl Into<AnyImageNode>,
        graph: &mut RenderGraph,
    ) {
        let src = src.into();
        let dst = dst.into();

        graph
            .begin_pass("linear_to_srgb")
            .bind_pipeline(&self.ppl)
            .read_descriptor((0, 0), src)
            .store_color(0, dst)
            .record_subpass(move |subpass, _| {
                subpass.draw(6, 1, 0, 0);
            });
    }
}
