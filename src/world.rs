use crate::accel::{Blas, BlasInfo, Tlas};
use crate::buffers::TypedBuffer;
use crate::dense_arena::{DenseArena, KeyData};
use crate::model::{
    GlslCamera, GlslInstanceData, GlslMaterial, Index, InstanceKey, Material, MaterialKey, Mesh,
    MeshInstance, MeshKey, ShaderGroup, ShaderGroupKey, ShaderKey, TextureKey, Vertex,
};
use crate::sbt::{SbtBuffer, SbtBufferInfo};

use bevy_ecs::prelude::*;
use bevy_ecs::system::CommandQueue;
use bevy_math::{Mat3, Mat4, Vec3, Vec4, Vec4Swizzles};
use bevy_transform::prelude::*;
use bytemuck::cast_slice;
use image::GenericImageView;
use screen_13::prelude::RayTracePipeline;
use screen_13::prelude::*;
use screen_13_fx::ImageLoader;
//use slotmap::*;
use crate::dense_arena::*;
use bitflags::*;
use std::collections::{BTreeMap, HashMap};
use std::io::BufReader;
use std::ops::{Deref, DerefMut, Range};
use std::path::Path;
use std::sync::Arc;

const INDEX_UNDEF: u32 = 0xffffffff;

#[derive(PartialEq, Eq)]
pub enum ResourceStatus {
    Recreated,
    //Changed,
    Unchanged,
}

pub struct Resource<T> {
    res: T,
    pub status: ResourceStatus,
}

impl<T> Resource<T> {
    pub fn new(res: T) -> Self {
        Self {
            res,
            status: ResourceStatus::Recreated,
        }
    }
    pub fn recreated(&self) -> bool {
        self.status == ResourceStatus::Recreated
    }
    pub fn into_inner(self) -> T {
        self.res
    }
    /*
    pub fn set_changed(&mut self) {
        self.status = ResourceStatus::Changed
    }
    pub fn changed(&self) -> bool {
        self.status == ResourceStatus::Changed
    }
    */
}

impl<T> Deref for Resource<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.res
    }
}
impl<T> DerefMut for Resource<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.res
    }
}

pub struct GpuScene {
    pub blases: HashMap<MeshKey, Resource<Blas>>,
    pub tlas: Option<Resource<Tlas>>,

    pub material_buf: Option<TypedBuffer<GlslMaterial>>,
    pub materials: DenseArena<MaterialKey, Resource<Material>>,

    pub instancedata_buf: Option<TypedBuffer<GlslInstanceData>>,
    pub instances: DenseArena<InstanceKey, Resource<MeshInstance>>,

    //pub shaders: DenseArena<ShaderKey, Shader>,

    // Resources. They are bound to the shader as seperate bindless buffers.
    pub textures: DenseArena<TextureKey, Arc<Image>>,

    pub mesh_bufs: DenseArena<MeshKey, Resource<Mesh>>,

    pub camera: Resource<GlslCamera>,

    pub shaders: DenseArena<ShaderKey, Resource<Shader>>,
    pub shader_groups: DenseArena<ShaderGroupKey, Resource<ShaderGroup>>,
    //pub shader_group_offsets: HashMap<ShaderGroupKey, u32>,
    pub pipeline: Option<Arc<RayTracePipeline>>,
    pub miss_groups: Vec<ShaderGroupKey>,
    pub rgen_group: Option<ShaderGroupKey>,
    pub hit_offsets: HashMap<InstanceKey, usize>,
    pub sbt: Option<SbtBuffer>,
}

impl GpuScene {
    pub fn recreate_stage(&mut self, device: &Arc<Device>) {
        let mut recreate_blases = false;
        let mut recreate_pipeline = false;
        let mut recreate_instance_buf = false;
        let mut recreate_material_buf = false;
        //let shaders = self.shaders.values().map(|s| s.clone()).collect::<Vec<_>>();
        recreate_pipeline |= self
            .shaders
            .values()
            .filter(|s| s.recreated())
            .next()
            .is_some();
        recreate_pipeline |= self
            .shader_groups
            .values()
            .filter(|s| s.recreated())
            .next()
            .is_some();
        if recreate_pipeline {
            self.recreate_pipeline(device);
        }
        // Recreate blases:
        for (key, mesh_buf) in self.mesh_bufs.iter() {
            if mesh_buf.recreated() {
                recreate_blases = true;
                self.blases.insert(
                    *key,
                    Resource::new(Blas::create(
                        device,
                        &BlasInfo {
                            indices: &mesh_buf.indices,
                            positions: &mesh_buf.vertices,
                        },
                    )),
                );
            }
        }
        // cleanup unused blases
        let remove_blases = self
            .blases
            .iter()
            .filter_map(|(mkey, _)| {
                if self.mesh_bufs.get(*mkey).is_none() {
                    Some(*mkey)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for mkey in remove_blases.iter() {
            self.blases.remove(&mkey);
        }
        recreate_material_buf |= self
            .materials
            .values()
            .filter(|mat| mat.recreated())
            .next()
            .is_some();
        if recreate_material_buf {
            self.recreate_material_buf(device);
        }
        recreate_instance_buf |= self
            .instances
            .values()
            .filter(|inst| inst.recreated())
            .next()
            .is_some();
        if recreate_instance_buf {
            self.recreate_sbt_buf(device);
            self.recreate_instance_buf(device);
        }
        if recreate_blases | recreate_instance_buf | recreate_pipeline {
            self.recreate_tlas(device);
        }
    }
    pub fn build_stage(&mut self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        let mut build_tlas = false;
        let blas_nodes = self
            .blases
            .iter()
            .map(|(_, b)| {
                if b.recreated() {
                    build_tlas = true;
                    b.build(cache, rgraph)
                } else {
                    AnyAccelerationStructureNode::AccelerationStructure(rgraph.bind_node(&b.accel))
                }
            })
            .collect::<Vec<_>>();
        if self.tlas.as_ref().unwrap().recreated() {
            build_tlas = true;
        }
        if build_tlas {
            self.tlas
                .as_ref()
                .unwrap()
                .build(cache, rgraph, &blas_nodes);
            self.camera.fc = 0;
            //println!("Rebuild TLAS");
        }
    }
    pub fn cleanup_stage(&mut self) {
        for shader in self.shaders.values_mut() {
            shader.status = ResourceStatus::Unchanged;
        }
        for group in self.shader_groups.values_mut() {
            group.status = ResourceStatus::Unchanged;
        }
        for material in self.materials.values_mut() {
            material.status = ResourceStatus::Unchanged;
        }
        for instance in self.instances.values_mut() {
            instance.status = ResourceStatus::Unchanged;
        }
        for mesh in self.mesh_bufs.values_mut() {
            mesh.status = ResourceStatus::Unchanged;
        }
        for blas in self.blases.values_mut() {
            blas.status = ResourceStatus::Unchanged;
        }
        self.tlas.as_mut().unwrap().status = ResourceStatus::Unchanged;
    }
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
        for (ikey, instance) in self.instances.iter() {
            if let Some(subset_idx) = subset_idx(&hit_keys, &instance.shader_groups) {
                hit_offsets.insert(*ikey, subset_idx);
            } else {
                hit_offsets.insert(*ikey, hit_keys.len());
                hit_keys.extend_from_slice(&instance.shader_groups);
            }
        }
        println!("{:#?}", hit_keys);
        let hit_indices = hit_keys
            .into_iter()
            .map(|k| self.shader_groups.dense_index(k))
            .collect::<Vec<_>>();
        let miss_indices = self
            .miss_groups
            .iter()
            .map(|k| self.shader_groups.dense_index(*k))
            .collect::<Vec<_>>();
        let rgen_index = self.shader_groups.dense_index(self.rgen_group.unwrap());
        let sbt_info = SbtBufferInfo {
            rgen_index,
            hit_indices: &hit_indices,
            miss_indices: &miss_indices,
            callable_indices: &[],
        };
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
                self.shaders
                    .values()
                    .map(|s| s.res.clone())
                    .collect::<Vec<_>>(),
                self.shader_groups
                    .values()
                    .map(|group| match group.res {
                        ShaderGroup::General { general } => RayTraceShaderGroup::new_general(
                            self.shaders.dense_index(general) as u32,
                        ),
                        ShaderGroup::Procedural {
                            intersection,
                            closest_hit,
                            any_hit,
                        } => RayTraceShaderGroup::new_procedural(
                            self.shaders.dense_index(intersection) as u32,
                            closest_hit.map(|ch| self.shaders.dense_index(ch) as u32),
                            any_hit.map(|ah| self.shaders.dense_index(ah) as u32),
                        ),
                        ShaderGroup::Triangle {
                            closest_hit,
                            any_hit,
                        } => RayTraceShaderGroup::new_triangles(
                            self.shaders.dense_index(closest_hit) as u32,
                            any_hit.map(|ah| self.shaders.dense_index(ah) as u32),
                        ),
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
        ))
    }
    fn recreate_tlas(&mut self, device: &Arc<Device>) {
        let mut instances = vec![];
        for (i, (key, instance)) in self.instances.iter().enumerate() {
            let matrix = instance.transform.compute_matrix();
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
        self.tlas = Some(Resource::new(Tlas::create(device, &instances)));
    }
    fn recreate_instance_buf(&mut self, device: &Arc<Device>) {
        let mut instancedata = vec![];
        for instance in self.instances.values_as_slice() {
            instancedata.push(GlslInstanceData {
                transform: instance.transform.compute_matrix().to_cols_array_2d(),
                //mat_index: self.materials[instance.material].index,
                mat_index: self.materials.dense_index(instance.material) as _,
                indices: self.mesh_bufs.dense_index(instance.mesh) as _,
                vertices: self.mesh_bufs.dense_index(instance.mesh) as _,
                normal_uv_mask: 0,
            });
        }
        self.instancedata_buf = Some(TypedBuffer::create(
            device,
            &instancedata,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        ));
    }
    fn recreate_material_buf(&mut self, device: &Arc<Device>) {
        let materials = self
            .materials
            .values()
            .map(|m| GlslMaterial {
                albedo: m.albedo,
                mr: m.mr,
                emission: [m.emission[0], m.emission[1], m.emission[2], 0.],
                transmission: m.transmission,
                transmission_roughness: m.transmission_roughness,
                ior: m.ior,
                _pack: 0,
                diffuse_tex: m
                    .albedo_tex
                    .map(|tex| self.textures.dense_index(tex) as _)
                    .unwrap_or(INDEX_UNDEF),
                mr_tex: m
                    .mr_tex
                    .map(|tex| self.textures.dense_index(tex) as _)
                    .unwrap_or(INDEX_UNDEF),
                emission_tex: m
                    .emission_tex
                    .map(|tex| self.textures.dense_index(tex) as _)
                    .unwrap_or(INDEX_UNDEF),
                normal_tex: m
                    .normal_tex
                    .map(|tex| self.textures.dense_index(tex) as _)
                    .unwrap_or(INDEX_UNDEF),
            })
            .collect::<Vec<_>>();
        self.material_buf = Some(TypedBuffer::create(
            device,
            &materials,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        ));
    }
    pub fn set_camera(&mut self, camera: GlslCamera) {
        self.camera = Resource::new(camera);
    }
    pub fn insert_shader(&mut self, shader: Shader) -> ShaderKey {
        self.shaders.insert(Resource::new(shader))
    }
    pub fn insert_shader_group(&mut self, group: ShaderGroup) -> ShaderGroupKey {
        self.shader_groups.insert(Resource::new(group))
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
        let mut img_loader = ImageLoader::new(device).unwrap();
        let img = img.as_rgba8().unwrap();
        let img = img_loader
            .decode_linear(
                img,
                screen_13_fx::ImageFormat::R8G8B8A8,
                img.width(),
                img.height(),
            )
            .unwrap();
        self.textures.insert(img)
    }
    pub fn insert_material(&mut self, material: Material) -> MaterialKey {
        self.materials.insert(Resource::new(material))
    }
    pub fn insert_instance(&mut self, instance: MeshInstance) -> InstanceKey {
        self.instances.insert(Resource::new(instance))
    }
    pub fn insert_mesh(
        &mut self,
        device: &Arc<Device>,
        indices: &[Index],
        vertices: &[Vertex],
    ) -> MeshKey {
        self.mesh_bufs.insert(Resource::new(Mesh {
            indices: Arc::new(TypedBuffer::create(
                device,
                indices,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            )),
            vertices: Arc::new(TypedBuffer::create(
                device,
                vertices,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            )),
        }))
    }
    pub fn new() -> Self {
        let camera = Resource::new(GlslCamera {
            up: [0., 0., 1., 1.],
            right: [0., 1., 0., 1.],
            pos: [1., 0., 0., 1.],
            focus: 1.,
            diameter: 0.1,
            fov: 1.,
            fc: 0,
            depth: 16,
        });
        Self {
            blases: HashMap::new(),
            tlas: None,
            material_buf: None,
            instancedata_buf: None,
            instances: DenseArena::default(),
            mesh_bufs: DenseArena::default(),
            materials: DenseArena::default(),
            textures: DenseArena::default(),
            camera,
            shaders: DenseArena::default(),
            shader_groups: DenseArena::default(),
            miss_groups: Vec::new(),
            rgen_group: None,
            hit_offsets: HashMap::default(),
            //shader_group_offsets: HashMap::default(),
            pipeline: None,
            sbt: None,
        }
    }
    pub fn append_gltf(&mut self, device: &Arc<Device>, default_hit_groups: Vec<ShaderGroupKey>) {
        let path = "./src/res/cube_scene.gltf";
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
                    albedo: mr.base_color_factor(),
                    mr: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                    emission,
                    transmission,
                    transmission_roughness: 0.,
                    ior,
                    albedo_tex,
                    mr_tex,
                    emission_tex,
                    normal_tex,
                    transmission_tex,
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
                    self.insert_instance(MeshInstance {
                        transform: Transform::from_matrix(Mat4::from_cols_array_2d(&matrix)),
                        material: material_entities[&mesh
                            .primitives()
                            .next()
                            .unwrap()
                            .material()
                            .index()
                            .unwrap()],
                        mesh: mesh_entities[&mesh.index()],
                        shader_groups: default_hit_groups.clone(),
                    });
                }
            }
        }
    }
}
