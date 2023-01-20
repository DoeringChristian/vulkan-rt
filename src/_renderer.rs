use crate::accel::{Blas, BlasInfo, Tlas};
use crate::array::{Array, SliceBuffer};
use crate::dense_arena::{DenseArena, KeyData};
use crate::gbuffer::GBuffer;
use crate::glsl;
use crate::model::{
    GlslCamera, Index, InstanceKey, Light, LightKey, Material, MaterialKey, Medium, Mesh,
    MeshInstance, MeshKey, PushConstant, ShaderGroup, ShaderGroupKey, ShaderKey, TextureKey,
    Vertex,
};
use crate::render_world::RenderWorld;
use crate::sbt::{SbtBuffer, SbtBufferInfo};

use bytemuck::cast_slice;
use screen_13::prelude::RayTracePipeline;
use screen_13::prelude::*;
//use slotmap::*;
use crate::dense_arena::*;
use glam::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

const INDEX_UNDEF: u32 = 0xffffffff;

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    MeshChanged(MeshKey),
    MeshResized(MeshKey),
    BlasRecreated(MeshKey),
    TlasRecreated,
    InstancesChanged,
    InstancesResized,
    MaterialsChanged,
    MaterialsResized,
    LightChanged,
    LightResized,
    TexturesChanged,
    TexturesResized,
    ShadersChanged,
    ShadersResized,
    ShaderGroupsChanged,
    ShaderGroupsResized,
    CameraChanged,
}

pub struct RTRenderer {
    pub blases: HashMap<MeshKey, Blas>,
    pub tlas: Option<Tlas>,

    pub material_buf: Option<Array<glsl::MaterialData>>,
    pub instancedata_buf: Option<Array<glsl::InstanceData>>,
    pub lightdata_buf: Option<SliceBuffer<glsl::LightData>>,

    pub pipeline: Option<Arc<RayTracePipeline>>,
    pub hit_offsets: HashMap<InstanceKey, usize>,
    pub miss_groups: Vec<ShaderGroupKey>,
    pub rgen_group: Option<ShaderGroupKey>,
    pub sbt: Option<SbtBuffer>,

    pub world: RenderWorld,
    pub signals: Mutex<HashSet<Signal>>,
}
mod bindings {
    pub const TLAS: (u32, u32) = (0, 0);
    pub const INSTANCES: (u32, u32) = (0, 1);
    pub const MATERIALS: (u32, u32) = (0, 2);
    pub const TEXTURES: (u32, u32) = (0, 3);
    pub const LIGHTS: (u32, u32) = (0, 4);
    pub const COLOR: (u32, u32) = (1, 0);
}

impl RTRenderer {
    pub fn emit(&self, signal: Signal) {
        self.signals.lock().unwrap().insert(signal);
    }
    #[inline]
    fn signaled(&self, signal: &Signal) -> bool {
        self.signals.lock().unwrap().contains(signal)
    }
    fn clear_signals(&self) {
        self.signals.lock().unwrap().clear();
    }
    pub fn recreate_stage(&mut self, device: &Arc<Device>) {
        if self.signaled(&Signal::CameraChanged) {
            self.world.camera.fc = 0;
        }
        if self.signaled(&Signal::LightChanged)
            || self.signaled(&Signal::LightResized)
            || self.lightdata_buf.is_none()
        {
            self.recreate_lightdata_buf(device);
        }
        if self.signaled(&Signal::ShadersResized)
            || self.signaled(&Signal::ShaderGroupsResized)
            || self.signaled(&Signal::ShadersChanged)
        {
            self.recreate_pipeline(device);
        }
        let mut recreate_blases = false;
        // Recreate blases:
        let mut blases = HashMap::new();
        let mut blas_signals = Vec::new();
        for (key, mesh) in self.world.meshes.iter() {
            //if self.any_signal([Signal::MeshResized(*key)].iter()) || !self.blases.contains_key(key)
            if self.signaled(&Signal::MeshResized(*key)) || !self.blases.contains_key(key) {
                recreate_blases = true;
                blas_signals.push(Signal::BlasRecreated(*key));
                blases.insert(
                    *key,
                    Blas::create(
                        device,
                        &BlasInfo {
                            indices: &mesh.indices,
                            positions: &mesh.vertices,
                        },
                    ),
                );
            } else {
                blases.insert(*key, self.blases.remove(key).unwrap());
            }
        }
        blas_signals.into_iter().for_each(|sig| self.emit(sig));
        self.blases = blases;
        if self.signaled(&Signal::MaterialsResized) || self.signaled(&Signal::MaterialsChanged) {
            self.recreate_material_buf(device);
        }
        if self.signaled(&Signal::InstancesResized) || self.signaled(&Signal::MaterialsResized) {
            self.recreate_sbt_buf(device);
            self.recreate_instancedata_buf(device);
        }
        if recreate_blases
            || self.signaled(&Signal::InstancesChanged)
            || self.signaled(&Signal::InstancesResized)
        {
            self.emit(Signal::TlasRecreated);
            self.recreate_tlas(device);
        }
    }
    pub fn build_stage(&mut self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        let mut build_tlas = false;
        let blas_nodes = self
            .blases
            .iter()
            .map(|(key, b)| {
                if self.signaled(&Signal::MeshChanged(*key))
                    || self.signaled(&Signal::MeshResized(*key))
                    || self.signaled(&Signal::BlasRecreated(*key))
                {
                    build_tlas = true;
                    b.build(cache, rgraph)
                } else {
                    AnyAccelerationStructureNode::AccelerationStructure(rgraph.bind_node(&b.accel))
                }
            })
            .collect::<Vec<_>>();
        if build_tlas || self.signaled(&Signal::TlasRecreated) {
            self.tlas.as_ref().and_then(|tlas| {
                tlas.build(cache, rgraph, &blas_nodes);
                Some(())
            });
            self.world.camera.fc = 0;
            //println!("Rebuild TLAS");
        }
    }
    pub fn cleanup_stage(&mut self) {
        self.clear_signals();
    }
}

impl RTRenderer {
    fn recreate_sbt_buf(&mut self, device: &Arc<Device>) {
        let mut hit_keys = vec![];
        let mut hit_offsets = HashMap::new();
        // Helper function to calculate the index of the first subset of a slice.
        fn subset_idx<T: Eq>(superset: &[T], subset: &[T]) -> Option<usize> {
            if superset.len() < subset.len() {
                return None;
            }
            for i in 0..(superset.len() - subset.len() + 1) {
                let mut is_subset = true;
                for j in 0..subset.len() {
                    if superset[i + j] != subset[j] {
                        is_subset = false;
                        println!("never");
                        break;
                    }
                }
                if is_subset {
                    return Some(i);
                }
            }
            None
        }
        for (ikey, instance) in self.world.instances.iter() {
            if let Some(subset_idx) = subset_idx(&hit_keys, &instance.shader_groups) {
                hit_offsets.insert(*ikey, subset_idx);
            } else {
                hit_offsets.insert(*ikey, hit_keys.len());
                hit_keys.extend_from_slice(&instance.shader_groups);
            }
        }
        let hit_indices = hit_keys
            .into_iter()
            .map(|k| self.world.shader_groups.dense_index(k))
            .collect::<Vec<_>>();
        let miss_indices = self
            .miss_groups
            .iter()
            .map(|k| self.world.shader_groups.dense_index(*k))
            .collect::<Vec<_>>();
        let rgen_index = self
            .world
            .shader_groups
            .dense_index(self.rgen_group.unwrap());
        let sbt_info = SbtBufferInfo {
            rgen_index,
            hit_indices: &hit_indices,
            miss_indices: &miss_indices,
            callable_indices: &[],
        };
        //println!("SbtBufferInfo: {:#?}", sbt_info);
        self.sbt =
            Some(SbtBuffer::create(device, sbt_info, &self.pipeline.as_ref().unwrap()).unwrap());
        self.hit_offsets = hit_offsets;
    }
    fn recreate_pipeline(&mut self, device: &Arc<Device>) {
        self.pipeline = Some(Arc::new(
            RayTracePipeline::create(
                device,
                RayTracePipelineInfo::new()
                    .max_ray_recursion_depth(3)
                    .build(),
                self.world
                    .shaders
                    .values()
                    .map(|s| s.clone())
                    .collect::<Vec<_>>(),
                self.world
                    .shader_groups
                    .values()
                    .map(|group| match group {
                        ShaderGroup::General { general } => RayTraceShaderGroup::new_general(
                            self.world.shaders.dense_index(*general) as u32,
                        ),
                        ShaderGroup::Procedural {
                            intersection,
                            closest_hit,
                            any_hit,
                        } => RayTraceShaderGroup::new_procedural(
                            self.world.shaders.dense_index(*intersection) as u32,
                            closest_hit.map(|ch| self.world.shaders.dense_index(ch) as u32),
                            any_hit.map(|ah| self.world.shaders.dense_index(ah) as u32),
                        ),
                        ShaderGroup::Triangle {
                            closest_hit,
                            any_hit,
                        } => RayTraceShaderGroup::new_triangles(
                            self.world.shaders.dense_index(*closest_hit) as u32,
                            any_hit.map(|ah| self.world.shaders.dense_index(ah) as u32),
                        ),
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        ))
    }
    fn recreate_tlas(&mut self, device: &Arc<Device>) {
        let mut instances = vec![];
        for (i, (key, instance)) in self.world.instances.iter().enumerate() {
            let matrix = instance.transform;
            let matrix = [
                matrix.x_axis.x,
                matrix.y_axis.x,
                matrix.z_axis.x,
                matrix.w_axis.x,
                matrix.x_axis.y,
                matrix.y_axis.y,
                matrix.z_axis.y,
                matrix.w_axis.y,
                matrix.x_axis.z,
                matrix.y_axis.z,
                matrix.z_axis.z,
                matrix.w_axis.z,
            ];
            instances.push(vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR { matrix },
                instance_custom_index_and_mask: vk::Packed24_8::new(i as _, 0xff),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    *self.hit_offsets.get(key).unwrap_or(&0) as u32,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                ),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: AccelerationStructure::device_address(
                        &self.blases[&instance.mesh].accel,
                    ),
                },
            });
        }
        //println!("Instances: {}", instances.len());
        //trace!("Instances: {}\n\n\n\n\n\n", instances.len());
        self.tlas = Tlas::create(device, &instances);
    }
    fn recreate_instancedata_buf(&mut self, device: &Arc<Device>) {
        let mut instancedata = vec![];
        for instance in self.world.instances.values_as_slice() {
            let mat = instance.transform.to_cols_array_2d();
            instancedata.push(glsl::InstanceData {
                trans0: std140::vec4(mat[0][0], mat[0][1], mat[0][2], mat[0][3]),
                trans1: std140::vec4(mat[1][0], mat[1][1], mat[1][2], mat[1][3]),
                trans2: std140::vec4(mat[2][0], mat[2][1], mat[2][2], mat[2][3]),
                trans3: std140::vec4(mat[3][0], mat[3][1], mat[3][2], mat[3][3]),
                mat_index: std140::uint(self.world.materials.dense_index(instance.material) as _),
                //mesh_index: std140::uint(self.world.meshes.dense_index(instance.mesh) as _),
                indices: glsl::uint64_t(Buffer::device_address(
                    &self.world.meshes[instance.mesh].indices.buf,
                )),
                vertices: glsl::uint64_t(Buffer::device_address(
                    &self.world.meshes[instance.mesh].vertices.buf,
                )),
            });
        }
        self.instancedata_buf = Some(Array::from_slice(
            device,
            &instancedata,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        ));
    }
    fn recreate_lightdata_buf(&mut self, device: &Arc<Device>) {
        let lights = self
            .world
            .lights
            .values()
            .map(|l| glsl::LightData::from(*l))
            .collect::<Vec<_>>();
        self.lightdata_buf = Some(SliceBuffer::create(
            device,
            &lights,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        ));
    }
    fn recreate_material_buf(&mut self, device: &Arc<Device>) {
        let materials = self
            .world
            .materials
            .values()
            .map(|m| glsl::MaterialData {
                albedo: std140::vec4(m.albedo[0], m.albedo[1], m.albedo[2], m.albedo[3]),
                emission: std140::vec4(m.emission[0], m.emission[1], m.emission[2], 1.),
                metallic: std140::float(m.metallic),
                roughness: std140::float(m.roughness),
                transmission: std140::float(m.transmission),
                transmission_roughness: std140::float(m.transmission_roughness),
                ior: std140::float(m.ior),
                albedo_tex: std140::uint(
                    m.albedo_tex
                        .map(|tex| self.world.textures.dense_index(tex) as _)
                        .unwrap_or(INDEX_UNDEF),
                ),
                mr_tex: std140::uint(
                    m.mr_tex
                        .map(|tex| self.world.textures.dense_index(tex) as _)
                        .unwrap_or(INDEX_UNDEF),
                ),
                emission_tex: std140::uint(
                    m.emission_tex
                        .map(|tex| self.world.textures.dense_index(tex) as _)
                        .unwrap_or(INDEX_UNDEF),
                ),
                normal_tex: std140::uint(
                    m.normal_tex
                        .map(|tex| self.world.textures.dense_index(tex) as _)
                        .unwrap_or(INDEX_UNDEF),
                ),
                med: glsl::MediumData {
                    color: std140::vec4(
                        m.medium.color.x,
                        m.medium.color.y,
                        m.medium.color.z,
                        m.medium.color.w,
                    ),
                    anisotropic: std140::float(m.medium.anisotropic),
                    density: std140::float(m.medium.density),
                },
            })
            .collect::<Vec<_>>();
        self.material_buf = Some(Array::from_slice(
            device,
            &materials,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        ));
    }
}
impl RTRenderer {
    pub fn set_camera(&mut self, camera: GlslCamera) {
        self.emit(Signal::CameraChanged);
        self.world.set_camera(camera)
    }
    pub fn get_camera(&self) -> GlslCamera {
        self.world.get_camera()
    }
    pub fn insert_shader(&mut self, shader: Shader) -> ShaderKey {
        self.emit(Signal::ShadersResized);
        self.world.insert_shader(shader)
    }
    pub fn insert_shader_group(&mut self, group: ShaderGroup) -> ShaderGroupKey {
        self.emit(Signal::ShaderGroupsResized);
        self.world.insert_shader_group(group)
    }
    pub fn set_miss_groups(&mut self, groups: Vec<ShaderGroupKey>) {
        self.miss_groups = groups;
    }
    pub fn set_rgen_group(&mut self, rgen: ShaderGroupKey) {
        self.rgen_group = Some(rgen);
    }
    pub fn insert_texture(
        &mut self,
        device: &Arc<Device>,
        img: &image::DynamicImage,
    ) -> TextureKey {
        self.emit(Signal::TexturesResized);
        self.world.insert_texture(device, img)
    }
    pub fn insert_material(&mut self, material: Material) -> MaterialKey {
        self.emit(Signal::MaterialsResized);
        self.world.insert_material(material)
    }
    pub fn insert_instance(&mut self, instance: MeshInstance) -> InstanceKey {
        self.emit(Signal::InstancesResized);
        self.world.insert_instance(instance)
    }
    pub fn insert_light(&mut self, light: Light) -> LightKey {
        self.emit(Signal::LightResized);
        self.world.insert_light(light)
    }
    pub fn insert_mesh(
        &mut self,
        device: &Arc<Device>,
        indices: &[Index],
        vertices: &[Vertex],
    ) -> MeshKey {
        let key = self.world.insert_mesh(device, indices, vertices);
        self.emit(Signal::MeshResized(key));
        key
    }
    pub fn new() -> Self {
        Self {
            blases: HashMap::new(),
            tlas: None,
            material_buf: None,
            instancedata_buf: None,
            lightdata_buf: None,
            hit_offsets: HashMap::default(),
            //shader_group_offsets: HashMap::default(),
            rgen_group: None,
            miss_groups: Vec::new(),
            pipeline: None,
            sbt: None,
            signals: Mutex::new(HashSet::new()),
            world: RenderWorld::default(),
        }
    }
}
impl RTRenderer {
    pub fn render(
        &mut self,
        //dst_img: impl Into<AnyImageNode>,
        gbuffer: &GBuffer,
        _cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) -> Option<()> {
        //let image_node = dst_img.into();
        //let image_info = rgraph.node_info(image_node);
        let color_image_node = rgraph.bind_node(&gbuffer.color);
        let width = gbuffer.size[0] as u32;
        let height = gbuffer.size[1] as u32;
        let blas_nodes = self
            .blases
            .iter()
            .map(|(_, b)| rgraph.bind_node(&b.accel))
            .collect::<Vec<_>>();
        let material_node = rgraph.bind_node(&self.material_buf.as_ref()?.buf);
        let lights_node = rgraph.bind_node(&self.lightdata_buf.as_ref()?.buf);
        let instancedata_node = rgraph.bind_node(&self.instancedata_buf.as_ref()?.buf);
        let tlas_node = rgraph.bind_node(&self.tlas.as_ref()?.accel);
        let sbt_node = rgraph.bind_node(self.sbt.as_ref()?.buffer());
        let texture_nodes = self
            .world
            .textures
            .values()
            .enumerate()
            .map(|(i, tex)| rgraph.bind_node(tex))
            .collect::<Vec<_>>();
        let mesh_nodes = self
            .world
            .meshes
            .values()
            .enumerate()
            .map(|(i, mesh)| {
                (
                    rgraph.bind_node(&mesh.indices.buf),
                    rgraph.bind_node(&mesh.vertices.buf),
                )
            })
            .collect::<Vec<_>>();
        let push_constant = PushConstant {
            camera: self.world.camera,
        };
        self.world.camera.fc += 1;

        let sbt_rgen = self.sbt.as_ref()?.rgen();
        let sbt_miss = self.sbt.as_ref()?.miss();
        let sbt_hit = self.sbt.as_ref()?.hit();
        let sbt_callable = self.sbt.as_ref()?.callable();

        let mut pass: PipelinePassRef<RayTracePipeline> = rgraph
            .begin_pass("RT pass")
            .bind_pipeline(self.pipeline.as_ref()?);
        for blas_node in blas_nodes {
            pass = pass.access_node(
                blas_node,
                AccessType::RayTracingShaderReadAccelerationStructure,
            );
        }
        pass = pass
            .access_node(sbt_node, AccessType::RayTracingShaderReadOther)
            .access_descriptor(
                bindings::TLAS,
                tlas_node,
                AccessType::RayTracingShaderReadAccelerationStructure,
            )
            .write_descriptor(bindings::COLOR, color_image_node)
            .read_descriptor(bindings::LIGHTS, lights_node)
            .read_descriptor(bindings::INSTANCES, instancedata_node)
            .read_descriptor(bindings::MATERIALS, material_node);

        for (_, (indices, vertices)) in mesh_nodes.into_iter().enumerate() {
            pass = pass.read_node(indices);
            pass = pass.read_node(vertices);
        }
        for (i, node) in texture_nodes.into_iter().enumerate() {
            pass =
                pass.read_descriptor((bindings::TEXTURES.0, bindings::TEXTURES.1, [i as _]), node);
        }
        pass.record_ray_trace(move |ray_trace| {
            ray_trace.push_constants(cast_slice(&[push_constant]));
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
        Some(())
    }
}
