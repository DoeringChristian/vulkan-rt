use inline_spirv::include_spirv;
use screen_13::prelude::*;
use std::sync::Arc;

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
                            include_spirv!("./src/shaders/quad_vert.glsl", vert).as_slice(),
                        ),
                        Shader::new_fragment(
                            include_spirv!("./src/shaders/linear_to_srgb_frag.glsl", frag)
                                .as_slice(),
                        ),
                    ],
                )
                .unwrap(),
            ),
        }
    }
    pub fn exec(
        &self,
        graph: &mut RenderGraph,
        src: impl Into<AnyImageNode>,
        dst: impl Into<AnyImageNode>,
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
