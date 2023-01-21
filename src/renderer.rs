use crate::glsl::PushConstant;
use crate::sbt::{SbtBuffer, SbtBufferInfo};
use crate::scene::Scene;
use crevice::std140::AsStd140;
use screen_13::prelude::*;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Default)]
pub struct PTRendererInfo<'a> {
    pub integrator: Option<&'a str>,
    pub bsdf: Option<&'a str>,
    pub sampler: Option<&'a str>,
    pub sensor: Option<&'a str>,
}

pub struct PTRenderer {
    sbt: SbtBuffer,
    ppl: Arc<RayTracePipeline>,
}

impl PTRenderer {
    pub fn new(device: &Arc<Device>, info: &PTRendererInfo) -> Self {
        let bindings = include_str!("shaders/path-tracing/bindings.glsl");
        let common = include_str!("shaders/path-tracing/common.glsl");
        let texture = include_str!("shaders/path-tracing/util/texture.glsl");
        let interaction = include_str!("shaders/path-tracing/interaction.glsl");
        let instance = include_str!("shaders/path-tracing/util/instance.glsl");
        let emitter = include_str!("shaders/path-tracing/util/emitter.glsl");
        let trace = include_str!("shaders/path-tracing/trace.glsl");
        let math = include_str!("shaders/path-tracing/util/math.glsl");
        let warp = include_str!("shaders/path-tracing/util/warp.glsl");
        let rand = include_str!("shaders/path-tracing/util/rand.glsl");
        let rgen = include_str!("shaders/path-tracing/rtx/rgen.glsl");

        let integrator = info
            .integrator
            .map(|path| fs::read_to_string(path).ok())
            .flatten()
            .unwrap_or(String::from(include_str!(
                "shaders/path-tracing/integrator/path.glsl"
            )));
        let sampler = info
            .sampler
            .map(|path| fs::read_to_string(path).ok())
            .flatten()
            .unwrap_or(String::from(include_str!(
                "shaders/path-tracing/sampler/independent.glsl"
            )));
        let sensor = info
            .sensor
            .map(|path| fs::read_to_string(path).ok())
            .flatten()
            .unwrap_or(String::from(include_str!(
                "shaders/path-tracing/sensor/perspective.glsl"
            )));
        let bsdf = info
            .bsdf
            .map(|path| fs::read_to_string(path).ok())
            .flatten()
            .unwrap_or(String::from(include_str!(
                "shaders/path-tracing/bsdf/diffuse.glsl"
            )));

        let (preamble, rgen) = rgen.lines().partition::<Vec<_>, _>(|line| {
            line.starts_with("#version") || line.starts_with("#extension")
        });
        let preamble = preamble.iter().fold(String::new(), |mut a, b| {
            writeln!(a, "{b}").unwrap();
            a
        });
        let rgen = rgen.iter().fold(String::new(), |mut a, b| {
            writeln!(a, "{b}").unwrap();
            a
        });

        let mut src = String::new();
        src.push_str(&preamble);
        src.push_str(rand);
        src.push_str(&sampler);
        src.push_str(math);
        src.push_str(warp);
        src.push_str(common);
        src.push_str(bindings);
        src.push_str(interaction);
        src.push_str(trace);

        src.push_str(texture);
        src.push_str(instance);
        src.push_str(emitter);
        src.push_str(&bsdf);
        src.push_str(&sensor);
        src.push_str(&integrator);
        src.push_str(&rgen);

        let src = src
            .lines()
            .filter(|line| !line.starts_with("#include"))
            .fold(String::new(), |mut a, b| {
                writeln!(a, "{b}").unwrap();
                a
            });

        let compiler = shaderc::Compiler::new().unwrap();
        let mut options = shaderc::CompileOptions::new().unwrap();
        options.set_target_spirv(shaderc::SpirvVersion::V1_5);

        let rgen_result = compiler
            .compile_into_spirv(
                &src,
                shaderc::ShaderKind::RayGeneration,
                "raygen.glsl",
                "main",
                Some(&options),
            )
            .unwrap();

        let ppl = Arc::new(
            RayTracePipeline::create(
                device,
                RayTracePipelineInfo::new()
                    .max_ray_recursion_depth(2)
                    .build(),
                [
                    Shader::new_ray_gen(
                        rgen_result.as_binary(),
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
        inline_spirv::include_spirv!("src/shaders/path-tracing/rtx/rgen.glsl", rmiss, vulkan1_2,
                                     I "src/shaders/path-tracing");
        let sbt = SbtBuffer::create(device, sbt_info, &ppl).unwrap();
        Self { sbt, ppl }
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

        pass = pass.write_descriptor((1, 0, [0]), image);

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
