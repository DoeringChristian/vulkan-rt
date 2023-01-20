use crate::glsl::PushConstant;
use crate::sbt::{SbtBuffer, SbtBufferInfo};
use crate::scene::Scene;
use crevice::std140::AsStd140;
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
                    //RayTraceShaderGroup::new_general(3),
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

    pub fn bind_and_render(
        &self,
        scene: &Scene,
        image: impl Into<AnyImageNode>,
        seed: u32,
        width: u32,
        height: u32,
        camera: u32,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) {
        let push_constant = PushConstant {
            camera,
            seed,
            max_depth: 8,
            rr_depth: 2,
        };

        let image = image.into();

        let accel = rgraph.bind_node(&scene.tlas.as_ref().unwrap().accel);
        let indices = rgraph.bind_node(&scene.index_data.as_ref().unwrap().buf);
        let positions = rgraph.bind_node(&scene.position_data.as_ref().unwrap().buf);
        let normals = rgraph.bind_node(&scene.normal_data.as_ref().unwrap().buf);
        let uvs = rgraph.bind_node(&scene.uv_data.as_ref().unwrap().buf);

        let instances = rgraph.bind_node(&scene.instance_data.as_ref().unwrap().buf);
        let meshes = rgraph.bind_node(&scene.mesh_data.as_ref().unwrap().buf);
        let emitters = rgraph.bind_node(&scene.emitter_data.as_ref().unwrap().buf);
        let materials = rgraph.bind_node(&scene.material_data.as_ref().unwrap().buf);
        let cameras = rgraph.bind_node(&scene.camera_data.as_ref().unwrap().buf);

        let textures = scene
            .textures_gpu
            .as_ref()
            .unwrap()
            .iter()
            .map(|texture| rgraph.bind_node(texture))
            .collect::<Vec<_>>();

        let mut pass = rgraph
            .begin_pass("Path Tracing Pass")
            .bind_pipeline(&self.ppl)
            .read_descriptor((0, 0), accel)
            .read_descriptor((0, 1), indices)
            .read_descriptor((0, 2), positions)
            .read_descriptor((0, 3), normals)
            .read_descriptor((0, 4), uvs)
            .read_descriptor((0, 5), instances)
            .read_descriptor((0, 6), meshes)
            .read_descriptor((0, 7), emitters)
            .read_descriptor((0, 8), materials)
            .read_descriptor((0, 9), cameras);

        for (i, texture) in textures.iter().enumerate() {
            pass = pass.read_descriptor((0, 10, [i as _]), *texture);
        }

        pass = pass.write_descriptor((1, 0), image);

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

        //pass.submit_pass();
    }
}
