use crate::common::PushConstant;
use crate::sbt::{SbtBuffer, SbtBufferInfo};
use crate::scene::{Scene, SceneBinding};
use crevice::std140::AsStd140;
use screen_13::prelude::*;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

macro_rules! new_pt {
    ($rgen:literal) => {
    pub fn new(device: &Arc<Device>) -> Self {
        let ppl = Arc::new(
            RayTracePipeline::create(
                device,
                RayTracePipelineInfo::new()
                    .max_ray_recursion_depth(2)
                    .build(),
                [
                    Shader::new_ray_gen(
                        inline_spirv::include_spirv!($rgen, rgen, vulkan1_2,
                                                     I "src/shaders/path-tracing")
                        .as_slice(),
                    ),
                    Shader::new_closest_hit(
                        inline_spirv::include_spirv!("src/shaders/path-tracing/rtx/rchit.glsl", rchit, vulkan1_2,
                                                     I "src/shaders/path-tracing")
                            .as_slice(),
                    ),
                    Shader::new_miss(
                        inline_spirv::include_spirv!("src/shaders/path-tracing/rtx/rmiss.glsl", rmiss, vulkan1_2,
                                                     I "src/shaders/path-tracing")
                            .as_slice(),
                    ),
                    Shader::new_miss(
                        inline_spirv::include_spirv!(
                            "src/shaders/path-tracing/rtx/rmiss_shadow.glsl",
                            rmiss,
                            vulkan1_2,
                            I "src/shaders/path-tracing"
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
            miss_indices: &[2, 3],
            callable_indices: &[],
        };
        let sbt = SbtBuffer::create(device, sbt_info, &ppl).unwrap();
        Self { sbt, ppl }
    }
    };
}

pub struct PTRenderer {
    sbt: SbtBuffer,
    ppl: Arc<RayTracePipeline>,
}

impl PTRenderer {
    new_pt!("src/shaders/path-tracing/rtx/rgen-diffuse.glsl");
    pub fn bind_and_render(
        &self,
        scene: &SceneBinding,
        //image: impl Into<AnyImageNode>,
        seed: u32,
        width: u32,
        height: u32,
        camera: u32,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) -> GBuffer {
        let push_constant = PushConstant {
            camera,
            seed,
            max_depth: 8,
            rr_depth: 2,
        };

        let mut lease_img = || -> AnyImageNode {
            let img = cache
                .lease(ImageInfo::new_2d(
                    vk::Format::R32G32B32A32_SFLOAT,
                    width,
                    height,
                    vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                ))
                .unwrap();
            rgraph.bind_node(img).into()
        };

        let color = lease_img();
        let position = lease_img();
        let normal = lease_img();

        let mut pass = rgraph
            .begin_pass("Path Tracing Pass")
            .bind_pipeline(&self.ppl)
            .read_descriptor((0, 0), scene.accel)
            .read_descriptor((0, 1), scene.indices)
            .read_descriptor((0, 2), scene.positions)
            .read_descriptor((0, 3), scene.normals)
            .read_descriptor((0, 4), scene.uvs)
            .read_descriptor((0, 5), scene.instances)
            .read_descriptor((0, 6), scene.meshes)
            .read_descriptor((0, 7), scene.emitters)
            .read_descriptor((0, 8), scene.materials)
            .read_descriptor((0, 9), scene.cameras);

        for (i, texture) in scene.textures.iter().enumerate() {
            pass = pass.read_descriptor((0, 10, [i as _]), *texture);
        }

        pass = pass.write_descriptor((1, 0, [0]), color);
        pass = pass.write_descriptor((1, 0, [1]), normal);
        pass = pass.write_descriptor((1, 0, [2]), position);

        let sbt_rgen = self.sbt.rgen();
        let sbt_miss = self.sbt.miss();
        let sbt_hit = self.sbt.hit();
        let sbt_callable = self.sbt.callable();

        pass.record_ray_trace(move |ray_trace, _| {
            ray_trace.push_constants(push_constant.as_std140().as_bytes());
            ray_trace.trace_rays(
                &sbt_rgen,
                &sbt_miss,
                &sbt_hit,
                &sbt_callable,
                width,
                height,
                1,
            );
        });

        GBuffer {
            color,
            normal,
            position,
        }
    }
}
pub struct GBuffer {
    pub color: AnyImageNode,
    pub normal: AnyImageNode,
    pub position: AnyImageNode,
}
