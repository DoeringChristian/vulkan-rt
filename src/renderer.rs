use crate::sbt::{SbtBuffer, SbtBufferInfo};
use screen_13::prelude::*;
use std::sync::Arc;

pub struct PTRenderer {
    device: Arc<Device>,
    sbt: SbtBuffer,
    ppl: Arc<RayTracePipeline>,
}

impl PTRenderer {
    pub fn create(device: &Arc<Device>) -> Self {
        let ppl = Arc::new(
            RayTracePipeline::create(
                device,
                RayTracePipelineInfo::new()
                    .max_ray_recursion_depth(2)
                    .build(),
                [
                    Shader::new_ray_gen(
                        inline_spirv::include_spirv!("src/shaders/rgen.glsl", rgen, vulkan1_2)
                            .as_slice(),
                    ),
                    Shader::new_closest_hit(
                        inline_spirv::include_spirv!("src/shaders/rchit.glsl", rchit, vulkan1_2)
                            .as_slice(),
                    ),
                    Shader::new_miss(
                        inline_spirv::include_spirv!("src/shaders/rmiss.glsl", rmiss, vulkan1_2)
                            .as_slice(),
                    ),
                    Shader::new_miss(
                        inline_spirv::include_spirv!(
                            "src/shaders/miss_shadow.glsl",
                            rmiss,
                            vulkan1_2
                        )
                        .as_slice(),
                    ),
                ],
                [
                    RayTraceShaderGroup::new_general(0),
                    RayTraceShaderGroup::new_triangles(1, None),
                    RayTraceShaderGroup::new_general(2),
                    RayTraceShaderGroup::new_general(3),
                ],
            )
            .unwrap(),
        );
        let sbt_info = SbtBufferInfo {
            rgen_index: 0,
            hit_indices: &[1],
            miss_indices: &[2],
            callable_indices: &[],
        };
        let sbt = SbtBuffer::create(device, sbt_info, &ppl).unwrap();
        Self {
            device: device.clone(),
            sbt,
            ppl,
        }
    }
}
