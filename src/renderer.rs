use crate::accel::{Blas, BlasInfo, Tlas};
use crate::buffers::{SliceBuffer, TypedBuffer};
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
use std::sync::Arc;

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

    pub material_buf: Option<TypedBuffer<glsl::MaterialData>>,
    pub instancedata_buf: Option<TypedBuffer<glsl::InstanceData>>,
    pub lightdata_buf: Option<SliceBuffer<glsl::LightData>>,

    pub pipeline: Option<Arc<RayTracePipeline>>,
    pub hit_offsets: HashMap<InstanceKey, usize>,
    pub miss_groups: Vec<ShaderGroupKey>,
    pub rgen_group: Option<ShaderGroupKey>,
    pub sbt: Option<SbtBuffer>,

    pub world: RenderWorld,
    pub signals: HashSet<Signal>,
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
    pub fn emit(&mut self, signal: Signal) {
        self.signals.insert(signal);
    }
    #[inline]
    fn signaled(&self, signal: &Signal) -> bool {
        self.signals.contains(signal)
    }
    fn clear_signals(&mut self) {
        self.signals.clear();
    }
    pub fn recreate_stage(&mut self, device: &Arc<Device>) {
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
            self.tlas
                .as_ref()
                .unwrap()
                .build(cache, rgraph, &blas_nodes);
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
        self.tlas = Some(Tlas::create(device, &instances));
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
        self.instancedata_buf = Some(TypedBuffer::create(
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
        self.material_buf = Some(TypedBuffer::create(
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
            signals: HashSet::new(),
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
    ) {
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
        let material_node = rgraph.bind_node(&self.material_buf.as_ref().unwrap().buf);
        let lights_node = rgraph.bind_node(&self.lightdata_buf.as_ref().unwrap().buf);
        let instancedata_node = rgraph.bind_node(&self.instancedata_buf.as_ref().unwrap().buf);
        let tlas_node = rgraph.bind_node(&self.tlas.as_ref().unwrap().accel);
        let sbt_node = rgraph.bind_node(self.sbt.as_ref().unwrap().buffer());
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

        let sbt_rgen = self.sbt.as_ref().unwrap().rgen();
        let sbt_miss = self.sbt.as_ref().unwrap().miss();
        let sbt_hit = self.sbt.as_ref().unwrap().hit();
        let sbt_callable = self.sbt.as_ref().unwrap().callable();

        let mut pass: PipelinePassRef<RayTracePipeline> = rgraph
            .begin_pass("RT pass")
            .bind_pipeline(self.pipeline.as_ref().unwrap());
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
    }
    pub fn append_gltf(
        &mut self,
        path: impl AsRef<Path>,
        device: &Arc<Device>,
        default_hit_groups: Vec<ShaderGroupKey>,
    ) -> Vec<InstanceKey> {
        //let path = "./src/res/cube_scene.gltf";
        let path = path.as_ref();
        let mut instances = vec![];
        let (gltf, buffers, _) = gltf::import(path).unwrap();
        {
            // Texture loading
            let mut texture_entities = HashMap::new();
            for texture in gltf.textures() {
                let image = match texture.source().source() {
                    gltf::image::Source::Uri { uri, mime_type } => {
                        let parent = Path::new(path).parent().unwrap();
                        let image_path = parent.join(uri);
                        let img = image::io::Reader::open(image_path)
                            .unwrap()
                            .decode()
                            .unwrap()
                            .into_rgba8();
                        image::DynamicImage::ImageRgba8(img)
                    }
                    _ => panic!("not supported"),
                };
                let entity = self.insert_texture(device, &image);
                texture_entities.insert(texture.index(), entity);
            }
            // Mesh loading
            let mut mesh_entities = HashMap::new();
            for mesh in gltf.meshes() {
                let primitive = mesh.primitives().next().unwrap();
                let mut indices = vec![];
                let mut vertices = vec![];
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let mut normal_iter = reader.read_normals();
                let mut uv0_iter = reader.read_tex_coords(0).map(|i| i.into_f32());
                let mut uv1_iter = reader.read_tex_coords(0).map(|i| i.into_f32());
                for pos in reader.read_positions().unwrap() {
                    let normal = normal_iter.as_mut().unwrap().next().unwrap_or([0., 0., 0.]);
                    let mut uv0 = [0., 0.];
                    let mut uv1 = [0., 0.];
                    if let Some(uv_iter) = uv0_iter.as_mut() {
                        uv0 = uv_iter.next().unwrap_or([0., 0.]);
                    }
                    if let Some(uv_iter) = uv1_iter.as_mut() {
                        uv1 = uv_iter.next().unwrap_or([0., 0.]);
                    }
                    vertices.push(Vertex {
                        pos: [pos[0], pos[1], pos[2], 1.],
                        normal: [normal[0], normal[1], normal[2], 0.],
                        uv01: [uv0[0], uv0[1], uv1[0], uv1[0]],
                    });
                }

                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        indices.push(Index(index));
                    }
                }
                let entity = self.insert_mesh(device, &indices, &vertices);
                mesh_entities.insert(mesh.index(), entity);
            }
            // Material loading
            let mut material_entities = HashMap::new();
            for material in gltf.materials() {
                let mr = material.pbr_metallic_roughness();
                let emission = material.emissive_factor();
                let albedo_tex = material
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map(|b| texture_entities[&b.texture().index()]);
                let mr_tex = material
                    .pbr_metallic_roughness()
                    .metallic_roughness_texture()
                    .map(|b| texture_entities[&b.texture().index()]);
                let emission_tex = material
                    .emissive_texture()
                    .map(|b| texture_entities[&b.texture().index()]);
                let normal_tex = material
                    .normal_texture()
                    .map(|b| texture_entities[&b.texture().index()]);
                let transmission = material
                    .transmission()
                    .map(|t| t.transmission_factor())
                    .unwrap_or(0.);
                let transmission_tex = material
                    .transmission()
                    .map(|t| {
                        t.transmission_texture()
                            .map(|t| texture_entities[&t.texture().index()])
                    })
                    .flatten();
                let ior = material.ior().unwrap_or(1.4);
                let material_entity = self.insert_material(Material {
                    albedo: Vec4::from(mr.base_color_factor()),
                    metallic: mr.metallic_factor(),
                    roughness: mr.roughness_factor(),
                    emission: Vec3::from(emission),
                    transmission,
                    transmission_roughness: 0.,
                    ior,
                    albedo_tex,
                    mr_tex,
                    emission_tex,
                    normal_tex,
                    transmission_tex,
                    medium: Medium {
                        color: Vec4::from(mr.base_color_factor()),
                        anisotropic: 0.,
                        density: 1. - transmission,
                    },
                });
                material_entities.insert(material.index().unwrap(), material_entity);
            }
            // Instance/Node and Camera loading
            for node in gltf.nodes() {
                if let Some(camera) = node.camera() {
                    if let gltf::camera::Projection::Perspective(proj) = camera.projection() {
                        let transform = Mat4::from_cols_array_2d(&node.transform().matrix());
                        let rot = Mat3::from_mat4(transform);
                        // Not quite sure about the default vectors.
                        let up = rot * Vec3::new(1., 0., 0.);
                        let right = rot * Vec3::new(0., -1., 0.);
                        let pos = transform * Vec4::new(0., 0., 0., 1.);

                        //self.world.insert_resource(camera);
                        let up = up.to_array();
                        let right = right.to_array();
                        let pos = pos.to_array();
                        self.set_camera(GlslCamera {
                            up: [up[0], up[1], up[2], 1.],
                            right: [right[0], right[1], right[2], 1.],
                            pos: [pos[0], pos[1], pos[2], 1.],
                            focus: 1.,
                            diameter: 0.1,
                            fov: proj.yfov(),
                            fc: 0,
                            depth: 16,
                        });
                    }
                }
                if let Some(mesh) = node.mesh() {
                    let matrix = node.transform().matrix();
                    instances.push(
                        self.insert_instance(MeshInstance {
                            transform: Mat4::from_cols_array_2d(&matrix),
                            material: material_entities[&mesh
                                .primitives()
                                .next()
                                .unwrap()
                                .material()
                                .index()
                                .unwrap()],
                            mesh: mesh_entities[&mesh.index()],
                            shader_groups: default_hit_groups.clone(),
                        }),
                    );
                }
                if let Some(light) = node.light() {
                    let transform = node.transform().matrix();
                    let pos = Mat4::from_cols_array_2d(&transform) * vec4(0., 0., 0., 1.);
                    self.insert_light(Light::Point {
                        emission: Vec3::from(light.color()),
                        position: pos.xyz(),
                        strength: light.intensity(),
                    });
                }
            }
        }
        // Dummy light
        //self.insert_light(Light::default());
        instances
    }
}
