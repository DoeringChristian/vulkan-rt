use crate::accel::{Blas, BlasInfo, Tlas};
use crate::buffers::TypedBuffer;
use crate::model::{Camera, GlslCamera, GlslInstanceData, GlslMaterial, Index, Vertex, Vertices};

use bevy_ecs::prelude::*;
use bevy_ecs::system::CommandQueue;
use bevy_math::{Mat3, Mat4, Vec3, Vec4, Vec4Swizzles};
use bevy_transform::prelude::*;
use bytemuck::cast_slice;
use image::GenericImageView;
use screen_13::prelude::*;
use screen_13_fx::ImageLoader;
use slotmap::*;
use std::collections::{BTreeMap, HashMap};
use std::io::BufReader;
use std::ops::{Deref, DerefMut, Range};
use std::path::Path;
use std::sync::Arc;

const INDEX_UNDEF: u32 = 0xffffffff;

pub struct GpuIndexed<T> {
    pub val: T,
    pub index: u32,
}

pub struct GpuIndexedSlotMap<K: Key, T> {
    map: SlotMap<K, GpuIndexed<T>>,
}

impl<K: Key, T> GpuIndexedSlotMap<K, T> {
    pub fn new() -> Self {
        Self {
            map: SlotMap::default(),
        }
    }

    pub fn insert(&mut self, val: T) -> K {
        self.map.insert(GpuIndexed {
            val,
            index: self.map.len() as _,
        })
    }
}

impl<T> Deref for GpuIndexed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}
impl<T> DerefMut for GpuIndexed<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

pub struct GpuInstance {
    pub transform: Transform,
    pub mesh: MeshKey,
    pub material: MaterialKey,
}

pub struct GpuMaterial {
    pub albedo: [f32; 4],
    pub mr: [f32; 4],
    pub emission: [f32; 3],
    pub albedo_tex: Option<TextureKey>,
    pub mr_tex: Option<TextureKey>,
    pub emission_tex: Option<TextureKey>,
    pub normal_tex: Option<TextureKey>,
}

new_key_type! {
    pub struct TextureKey;
    pub struct MeshKey;
    pub struct BlasKey;
    pub struct InstanceKey;
    pub struct MaterialKey;
}

pub struct GpuScene {
    // Maybee use hashmap
    pub blases: Vec<Blas>,
    pub tlas: Option<Tlas>,

    pub material_buf: Option<TypedBuffer<GlslMaterial>>,
    // maybee use dense slotmap
    pub materials: SlotMap<MaterialKey, GpuIndexed<GpuMaterial>>,
    pub material_count: u32,

    pub instancedata_buf: Option<TypedBuffer<GlslInstanceData>>,
    pub instances: SlotMap<InstanceKey, GpuInstance>,
    //pub instance_count: u32,

    // Assets:
    pub textures: SlotMap<TextureKey, GpuIndexed<Arc<Image>>>,
    pub texture_count: u32,

    pub mesh_bufs:
        SlotMap<MeshKey, GpuIndexed<(Arc<TypedBuffer<Index>>, Arc<TypedBuffer<Vertex>>)>>,
    pub mesh_count: u32,

    pub camera: GlslCamera,
}

impl GpuScene {
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        let blas_nodes = self
            .blases
            .iter()
            .map(|b| b.build(self, cache, rgraph))
            .collect::<Vec<_>>();
        self.tlas
            .as_ref()
            .unwrap()
            .build(cache, rgraph, &blas_nodes);
    }
    pub fn upload_data(&mut self, device: &Arc<Device>) {
        let mut materials = self.materials.values().collect::<Vec<_>>();
        materials.sort_by(|a, b| a.index.cmp(&b.index));
        let materials = materials
            .iter()
            .map(|m| GlslMaterial {
                albedo: m.albedo,
                mr: m.mr,
                emission: [m.emission[0], m.emission[1], m.emission[2], 0.],
                diffuse_tex: m
                    .albedo_tex
                    .map(|tex| self.textures[tex].index)
                    .unwrap_or(INDEX_UNDEF),
                mr_tex: m
                    .mr_tex
                    .map(|tex| self.textures[tex].index)
                    .unwrap_or(INDEX_UNDEF),
                emission_tex: m
                    .emission_tex
                    .map(|tex| self.textures[tex].index)
                    .unwrap_or(INDEX_UNDEF),
                normal_tex: m
                    .normal_tex
                    .map(|tex| self.textures[tex].index)
                    .unwrap_or(INDEX_UNDEF),
            })
            .collect::<Vec<_>>();
        self.material_buf = Some(TypedBuffer::create(
            device,
            &materials,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        ));
        let mut blases = HashMap::new();
        for (key, mesh_buf) in self.mesh_bufs.iter() {
            self.blases.push(Blas::create(
                device,
                &BlasInfo {
                    indices: &mesh_buf.val.0,
                    positions: &mesh_buf.val.1,
                },
            ));
            blases.insert(key, self.blases.len() - 1);
        }
        let mut instances = vec![];
        let mut instancedata = vec![];
        for instance in self.instances.values() {
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
                instance_custom_index_and_mask: vk::Packed24_8::new(
                    (instancedata.len()) as _,
                    0xff,
                ),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                ),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: AccelerationStructure::device_address(
                        &self.blases[blases[&instance.mesh]].accel,
                    ),
                },
            });
            instancedata.push(GlslInstanceData {
                transform: instance.transform.compute_matrix().to_cols_array_2d(),
                mat_index: self.materials[instance.material].index,
                indices: self.mesh_bufs[instance.mesh].index,
                vertices: self.mesh_bufs[instance.mesh].index,
                normal_uv_mask: 0,
            });
        }
        self.instancedata_buf = Some(TypedBuffer::create(
            device,
            &instancedata,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        ));
        self.tlas = Some(Tlas::create(device, &instances));
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
        let texture = self.textures.insert(GpuIndexed {
            val: img,
            index: self.texture_count,
        });
        self.texture_count += 1;
        texture
    }
    pub fn insert_material(&mut self, material: GpuMaterial) -> MaterialKey {
        let material = self.materials.insert(GpuIndexed {
            val: material,
            index: self.material_count,
        });
        self.material_count += 1;
        material
    }
    pub fn insert_instance(&mut self, instance: GpuInstance) -> InstanceKey {
        let instance = self.instances.insert(instance);
        instance
    }
    pub fn insert_mesh(
        &mut self,
        device: &Arc<Device>,
        indices: &[Index],
        vertices: &[Vertex],
    ) -> MeshKey {
        let mesh = self.mesh_bufs.insert(GpuIndexed {
            val: (
                Arc::new(TypedBuffer::create(
                    device,
                    indices,
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                )),
                Arc::new(TypedBuffer::create(
                    device,
                    vertices,
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                )),
            ),
            index: self.mesh_count,
        });
        self.mesh_count += 1;
        mesh
    }
    pub fn new() -> Self {
        let camera = GlslCamera {
            up: [0., 0., 1., 1.],
            right: [0., 1., 0., 1.],
            pos: [1., 0., 0., 1.],
            focus: 1.,
            diameter: 0.1,
            fov: 1.,
            fc: 0,
        };
        Self {
            blases: Vec::new(),
            tlas: None,
            material_buf: None,
            instancedata_buf: None,
            instances: SlotMap::default(),
            mesh_bufs: SlotMap::default(),
            mesh_count: 0,
            materials: SlotMap::default(),
            material_count: 0,
            textures: SlotMap::default(),
            texture_count: 0,
            camera,
        }
    }
    pub fn append_gltf(&mut self, device: &Arc<Device>) {
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
                let material_entity = self.insert_material(GpuMaterial {
                    albedo: mr.base_color_factor(),
                    mr: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                    emission,
                    albedo_tex,
                    mr_tex,
                    emission_tex,
                    normal_tex,
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

                        let camera = Camera {
                            up: up.to_array(),
                            right: right.to_array(),
                            pos: pos.xyz().to_array(),
                            focus: 1.,
                            diameter: 0.1,
                            fov: proj.yfov(),
                        };
                        trace!("Camera: {:#?}", camera);

                        //self.world.insert_resource(camera);
                        let up = up.to_array();
                        let right = right.to_array();
                        let pos = pos.to_array();
                        self.camera = GlslCamera {
                            up: [up[0], up[1], up[2], 1.],
                            right: [right[0], right[1], right[2], 1.],
                            pos: [pos[0], pos[1], pos[2], 1.],
                            focus: 1.,
                            diameter: 0.1,
                            fov: proj.yfov(),
                            fc: 0,
                        };
                    }
                }
                if let Some(mesh) = node.mesh() {
                    let matrix = node.transform().matrix();
                    self.insert_instance(GpuInstance {
                        transform: Transform::from_matrix(Mat4::from_cols_array_2d(&matrix)),
                        material: material_entities[&mesh
                            .primitives()
                            .next()
                            .unwrap()
                            .material()
                            .index()
                            .unwrap()],
                        mesh: mesh_entities[&mesh.index()],
                    });
                }
            }
        }
    }
}
