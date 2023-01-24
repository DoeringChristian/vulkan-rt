use crate::array::Array;
use crate::common::{PushConstant, RestirReservoir, RestirSample};
use crate::sbt::{SbtBuffer, SbtBufferInfo};
use crate::scene::{Scene, SceneBinding};
use crevice::std140::AsStd140;
use screen_13::prelude::*;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct GBuffer {
    pub color: AnyImageNode,
    pub normal: AnyImageNode,
    pub position: AnyImageNode,
}

pub struct RTPipeline {
    pub sbt: SbtBuffer,
    pub ppl: Arc<RayTracePipeline>,
}

impl RTPipeline {
    pub fn new(device: &Arc<Device>, rgen: &[u32]) -> Self {
        let ppl = Arc::new(
            RayTracePipeline::create(
                device,
                RayTracePipelineInfo::new()
                    .max_ray_recursion_depth(2)
                    .build(),
                [
                    Shader::new_ray_gen(
                        rgen,
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
}

pub struct PTRenderer {
    ppl: RTPipeline,
}

impl PTRenderer {
    pub fn new(device: &Arc<Device>) -> Self {
        Self {
            ppl: RTPipeline::new(
                device,
                inline_spirv::include_spirv!("src/shaders/path-tracing/integrator/path-gbuffer.glsl",
                                             rgen, vulkan1_2, 
                                             I "src/shaders/path-tracing").as_slice(),
            ),
        }
    }
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
            .bind_pipeline(&self.ppl.ppl)
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

        pass = pass.write_descriptor((1, 0), color);
        pass = pass.write_descriptor((1, 1), normal);
        pass = pass.write_descriptor((1, 2), position);

        let sbt_rgen = self.ppl.sbt.rgen();
        let sbt_miss = self.ppl.sbt.miss();
        let sbt_hit = self.ppl.sbt.hit();
        let sbt_callable = self.ppl.sbt.callable();

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

pub struct RestirRenderer{
    initial_ppl: RTPipeline,
    temporal_ppl: RTPipeline,
    spatial_ppl: RTPipeline,
    output_ppl: Arc<ComputePipeline>,
    initial_sample: Array<RestirSample>,
    temporal_reservoir: Array<RestirReservoir>,
    spatial_reservoir: Array<RestirReservoir>,
    width: usize,
    height: usize,
}
impl RestirRenderer{
    pub fn new(device: &Arc<Device>, width: usize, height: usize) -> Self{

        let initial_ppl = RTPipeline::new(device,
                                          inline_spirv::include_spirv!("src/shaders/path-tracing/integrator/restir-gi/restir-initial.glsl",
                                                                       rgen, vulkan1_2, 
                                                                       I "src/shaders/path-tracing").as_slice(),
                                                                       );
        let temporal_ppl = RTPipeline::new(device,
                                           inline_spirv::include_spirv!("src/shaders/path-tracing/integrator/restir-gi/restir-temporal.glsl",
                                                                        rgen, vulkan1_2, 
                                                                        I "src/shaders/path-tracing").as_slice(),
                                                                        );
        let spatial_ppl = RTPipeline::new(device,
                                          inline_spirv::include_spirv!("src/shaders/path-tracing/integrator/restir-gi/restir-spatial.glsl",
                                                                       rgen, vulkan1_2, 
                                                                       I "src/shaders/path-tracing").as_slice(),
                                                                       );

        let output_ppl = Arc::new(ComputePipeline::create(device, ComputePipelineInfo::default(),
        Shader::new_compute(
            inline_spirv::include_spirv!("src/shaders/path-tracing/integrator/restir-gi/restir-output.glsl",
                                             comp, vulkan1_2, 
                                             I "src/shaders/path-tracing",
                                             D COMPUTE).as_slice()
                    )).unwrap());
        
        let initial_sample = Array::uninitialized(device, vk::BufferUsageFlags::STORAGE_BUFFER, width * height);
        let temporal_reservoir = Array::uninitialized(device, vk::BufferUsageFlags::STORAGE_BUFFER, width * height);
        let spatial_reservoir = Array::uninitialized(device, vk::BufferUsageFlags::STORAGE_BUFFER, width * height);

        Self{
            initial_ppl,
            temporal_ppl,
            spatial_ppl,
            output_ppl,
            initial_sample,
            temporal_reservoir,
            spatial_reservoir,
            width: width as _,
            height: height as _,
        }
    }

    pub fn bind_and_render(
        &self,
        scene: &SceneBinding,
        seed: u32,
        camera: u32,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) -> AnyImageNode {

        let width = self.width as u32;
        let height = self.height as u32;
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

        let initial_sample = rgraph.bind_node(&self.initial_sample.buf);
        let temporal_reservoir = rgraph.bind_node(&self.temporal_reservoir.buf);
        let spatial_reservoir = rgraph.bind_node(&self.spatial_reservoir.buf);

        let mut pass = rgraph
            .begin_pass("Path Tracing Pass")
            .bind_pipeline(&self.initial_ppl.ppl)
            .read_descriptor((0, 0), scene.accel)
            .read_descriptor((0, 1), scene.indices)
            .read_descriptor((0, 2), scene.positions)
            .read_descriptor((0, 3), scene.normals)
            .read_descriptor((0, 4), scene.uvs)
            .read_descriptor((0, 5), scene.instances)
            .read_descriptor((0, 6), scene.meshes)
            .read_descriptor((0, 7), scene.emitters)
            .read_descriptor((0, 8), scene.materials)
            .read_descriptor((0, 9), scene.cameras)
            .write_descriptor((1, 0), initial_sample)
            .write_descriptor((1, 1), temporal_reservoir)
            .write_descriptor((1, 2), temporal_reservoir);

        for (i, texture) in scene.textures.iter().enumerate() {
            pass = pass.read_descriptor((0, 10, [i as _]), *texture);
        }

        let sbt_rgen = self.initial_ppl.sbt.rgen();
        let sbt_miss = self.initial_ppl.sbt.miss();
        let sbt_hit = self.initial_ppl.sbt.hit();
        let sbt_callable = self.initial_ppl.sbt.callable();

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
        
        let mut pass = rgraph
            .begin_pass("Path Tracing Pass")
            .bind_pipeline(&self.temporal_ppl.ppl)
            .read_descriptor((0, 0), scene.accel)
            .read_descriptor((0, 1), scene.indices)
            .read_descriptor((0, 2), scene.positions)
            .read_descriptor((0, 3), scene.normals)
            .read_descriptor((0, 4), scene.uvs)
            .read_descriptor((0, 5), scene.instances)
            .read_descriptor((0, 6), scene.meshes)
            .read_descriptor((0, 7), scene.emitters)
            .read_descriptor((0, 8), scene.materials)
            .read_descriptor((0, 9), scene.cameras)
            .write_descriptor((1, 0), initial_sample)
            .write_descriptor((1, 1), temporal_reservoir)
            .write_descriptor((1, 2), temporal_reservoir);

        for (i, texture) in scene.textures.iter().enumerate() {
            pass = pass.read_descriptor((0, 10, [i as _]), *texture);
        }

        let sbt_rgen = self.initial_ppl.sbt.rgen();
        let sbt_miss = self.initial_ppl.sbt.miss();
        let sbt_hit = self.initial_ppl.sbt.hit();
        let sbt_callable = self.initial_ppl.sbt.callable();

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
        
        let mut pass = rgraph
            .begin_pass("Path Tracing Pass")
            .bind_pipeline(&self.spatial_ppl.ppl)
            .read_descriptor((0, 0), scene.accel)
            .read_descriptor((0, 1), scene.indices)
            .read_descriptor((0, 2), scene.positions)
            .read_descriptor((0, 3), scene.normals)
            .read_descriptor((0, 4), scene.uvs)
            .read_descriptor((0, 5), scene.instances)
            .read_descriptor((0, 6), scene.meshes)
            .read_descriptor((0, 7), scene.emitters)
            .read_descriptor((0, 8), scene.materials)
            .read_descriptor((0, 9), scene.cameras)
            .write_descriptor((1, 0), initial_sample)
            .write_descriptor((1, 1), temporal_reservoir)
            .write_descriptor((1, 2), temporal_reservoir);

        for (i, texture) in scene.textures.iter().enumerate() {
            pass = pass.read_descriptor((0, 10, [i as _]), *texture);
        }

        let sbt_rgen = self.initial_ppl.sbt.rgen();
        let sbt_miss = self.initial_ppl.sbt.miss();
        let sbt_hit = self.initial_ppl.sbt.hit();
        let sbt_callable = self.initial_ppl.sbt.callable();

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
        
        let mut pass = rgraph
            .begin_pass("Path Tracing Pass")
            .bind_pipeline(&self.output_ppl)
            .read_descriptor((0, 0), scene.accel)
            .read_descriptor((0, 1), scene.indices)
            .read_descriptor((0, 2), scene.positions)
            .read_descriptor((0, 3), scene.normals)
            .read_descriptor((0, 4), scene.uvs)
            .read_descriptor((0, 5), scene.instances)
            .read_descriptor((0, 6), scene.meshes)
            .read_descriptor((0, 7), scene.emitters)
            .read_descriptor((0, 8), scene.materials)
            .read_descriptor((0, 9), scene.cameras)
            .write_descriptor((1, 0), initial_sample)
            .write_descriptor((1, 1), temporal_reservoir)
            .write_descriptor((1, 2), temporal_reservoir)
            .write_descriptor((1, 3), color);

        for (i, texture) in scene.textures.iter().enumerate() {
            pass = pass.read_descriptor((0, 10, [i as _]), *texture);
        }

        pass.record_compute(move |compute, _|{
            compute.push_constants(push_constant.as_std140().as_bytes());
            compute.dispatch(width, height, 1);
        });

        color
    }
    
}


// pub struct RestirRenderer {
//     initial_ppl: RTPipeline,
//     temporal_ppl: RTPipeline,
//     spatial_ppl: RTPipeline,
//     initial_sample: Array<Sample>,
//     temporal_reservoir: Array<Reservoir>,
//     spatial_reservoir: Array<Reservoir>,
//     width: usize,
//     height: usize,
// }
//
// impl RestirRenderer {
//     pub fn new(device: &Arc<Device>, width: usize, height: usize) -> Self {
//         let ppl = Arc::new(
//             RayTracePipeline::create(
//                 device,
//                 RayTracePipelineInfo::new()
//                     .max_ray_recursion_depth(2)
//                     .build(),
//                 [
//                     Shader::new_ray_gen(
//                         inline_spirv::include_spirv!("src/shaders/path-tracing/rtx/rgen-restir.glsl", rgen, vulkan1_2,
//                                                      I "src/shaders/path-tracing")
//                         .as_slice(),
//                     ),
//                     Shader::new_closest_hit(
//                         inline_spirv::include_spirv!("src/shaders/path-tracing/rtx/rchit.glsl", rchit, vulkan1_2,
//                                                      I "src/shaders/path-tracing")
//                             .as_slice(),
//                     ),
//                     Shader::new_miss(
//                         inline_spirv::include_spirv!("src/shaders/path-tracing/rtx/rmiss.glsl", rmiss, vulkan1_2,
//                                                      I "src/shaders/path-tracing")
//                             .as_slice(),
//                     ),
//                     Shader::new_miss(
//                         inline_spirv::include_spirv!(
//                             "src/shaders/path-tracing/rtx/rmiss_shadow.glsl",
//                             rmiss,
//                             vulkan1_2,
//                             I "src/shaders/path-tracing"
//                         )
//                         .as_slice(),
//                     ),
//                 ],
//                 [
//                     RayTraceShaderGroup::new_general(0),
//                     RayTraceShaderGroup::new_triangles(1, None),
//                     RayTraceShaderGroup::new_general(2),
//                     RayTraceShaderGroup::new_general(3),
//                 ],
//             )
//             .unwrap(),
//         );
//         let sbt_info = SbtBufferInfo {
//             rgen_index: 0,
//             hit_indices: &[1],
//             miss_indices: &[2, 3],
//             callable_indices: &[],
//         };
//         let sbt = SbtBuffer::create(device, sbt_info, &ppl).unwrap();
//         let initial_sample =
//             Array::uninitialized(device, vk::BufferUsageFlags::STORAGE_BUFFER, width * height);
//         let temporal_reservoir =
//             Array::uninitialized(device, vk::BufferUsageFlags::STORAGE_BUFFER, width * height);
//         let spatial_reservoir =
//             Array::uninitialized(device, vk::BufferUsageFlags::STORAGE_BUFFER, width * height);
//         Self {
//             width,
//             height,
//             initial_sample,
//             spatial_reservoir,
//             temporal_reservoir,
//         }
//     }
//     pub fn bind_and_render(
//         &self,
//         scene: &SceneBinding,
//         seed: u32,
//         camera: u32,
//         cache: &mut HashPool,
//         rgraph: &mut RenderGraph,
//     ) -> GBuffer {
//         let width = self.width as u32;
//         let height = self.height as u32;
//         let push_constant = PushConstant {
//             camera,
//             seed,
//             max_depth: 8,
//             rr_depth: 2,
//         };
//
//         let mut lease_img = || -> AnyImageNode {
//             let img = cache
//                 .lease(ImageInfo::new_2d(
//                     vk::Format::R32G32B32A32_SFLOAT,
//                     width,
//                     height,
//                     vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
//                 ))
//                 .unwrap();
//             rgraph.bind_node(img).into()
//         };
//
//         let color = lease_img();
//         let position = lease_img();
//         let normal = lease_img();
//
//         let mut pass = rgraph
//             .begin_pass("Path Tracing Pass")
//             .bind_pipeline(&self.ppl)
//             .read_descriptor((0, 0), scene.accel)
//             .read_descriptor((0, 1), scene.indices)
//             .read_descriptor((0, 2), scene.positions)
//             .read_descriptor((0, 3), scene.normals)
//             .read_descriptor((0, 4), scene.uvs)
//             .read_descriptor((0, 5), scene.instances)
//             .read_descriptor((0, 6), scene.meshes)
//             .read_descriptor((0, 7), scene.emitters)
//             .read_descriptor((0, 8), scene.materials)
//             .read_descriptor((0, 9), scene.cameras);
//
//         for (i, texture) in scene.textures.iter().enumerate() {
//             pass = pass.read_descriptor((0, 10, [i as _]), *texture);
//         }
//
//         pass = pass.write_descriptor((1, 0), color);
//         pass = pass.write_descriptor((1, 1), normal);
//         pass = pass.write_descriptor((1, 2), position);
//
//         let sbt_rgen = self.sbt.rgen();
//         let sbt_miss = self.sbt.miss();
//         let sbt_hit = self.sbt.hit();
//         let sbt_callable = self.sbt.callable();
//
//         pass.record_ray_trace(move |ray_trace, _| {
//             ray_trace.push_constants(push_constant.as_std140().as_bytes());
//             ray_trace.trace_rays(
//                 &sbt_rgen,
//                 &sbt_miss,
//                 &sbt_hit,
//                 &sbt_callable,
//                 width,
//                 height,
//                 1,
//             );
//         });
//
//         GBuffer {
//             color,
//             normal,
//             position,
//         }
//     }
// }
