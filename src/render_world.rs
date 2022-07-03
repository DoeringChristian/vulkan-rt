use crate::{
    buffers::TypedBuffer,
    dense_arena::DenseArena,
    model::{
        GlslCamera, Index, InstanceKey, Material, MaterialKey, Mesh, MeshInstance, MeshKey,
        ShaderGroup, ShaderGroupKey, ShaderKey, TextureKey, Vertex,
    },
};
use glam::*;
use screen_13::prelude::*;
use screen_13_fx::ImageLoader;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
pub enum RenderWorldEvent {
    MeshChanged(MeshKey),
    MeshResized(MeshKey),
    InstancesChanged,
    InstancesResized,
    MaterialsChanged,
    MaterialsResized,
    TexturesChanged,
    TexturesResized,
    ShadersChanged,
    ShadersResized,
    ShaderGroupsChanged,
    ShaderGroupsResized,
    CameraChanged,
}

#[derive(Default)]
pub struct RenderWorld {
    pub meshes: DenseArena<MeshKey, Mesh>,
    pub textures: DenseArena<TextureKey, Arc<Image>>,
    pub materials: DenseArena<MaterialKey, Material>,
    pub instances: DenseArena<InstanceKey, MeshInstance>,
    pub shaders: DenseArena<ShaderKey, Shader>,
    pub shader_groups: DenseArena<ShaderGroupKey, ShaderGroup>,
    pub camera: GlslCamera,
    pub events: HashSet<RenderWorldEvent>,
}

impl RenderWorld {
    pub fn reset_events(&mut self) {
        self.events.clear();
    }
    fn set_event(&mut self, event: RenderWorldEvent) {
        self.events.insert(event);
    }
    pub fn event_called(&self, event: &RenderWorldEvent) -> bool {
        self.events.contains(event)
    }
    pub fn any_event(&self, events: impl Iterator<Item = RenderWorldEvent>) -> bool {
        for event in events {
            if self.events.contains(&event) {
                return true;
            }
        }
        false
    }
    pub fn set_camera(&mut self, camera: GlslCamera) {
        self.set_event(RenderWorldEvent::CameraChanged);
        self.camera = camera;
    }
    pub fn insert_shader(&mut self, shader: Shader) -> ShaderKey {
        self.set_event(RenderWorldEvent::ShadersResized);
        self.shaders.insert(shader)
    }
    pub fn insert_shader_group(&mut self, group: ShaderGroup) -> ShaderGroupKey {
        self.set_event(RenderWorldEvent::ShaderGroupsResized);
        self.shader_groups.insert(group)
    }
    pub fn insert_texture(
        &mut self,
        device: &Arc<Device>,
        img: &image::DynamicImage,
    ) -> TextureKey {
        self.set_event(RenderWorldEvent::TexturesResized);
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
        self.set_event(RenderWorldEvent::MaterialsResized);
        self.materials.insert(material)
    }
    pub fn insert_instance(&mut self, instance: MeshInstance) -> InstanceKey {
        self.set_event(RenderWorldEvent::InstancesResized);
        self.instances.insert(instance)
    }
    pub fn insert_mesh(
        &mut self,
        device: &Arc<Device>,
        indices: &[Index],
        vertices: &[Vertex],
    ) -> MeshKey {
        let key = self.meshes.insert(Mesh {
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
        });
        self.set_event(RenderWorldEvent::MeshResized(key));
        key
    }
    pub fn append_gltf(
        &mut self,
        device: &Arc<Device>,
        default_hit_groups: Vec<ShaderGroupKey>,
    ) -> Vec<InstanceKey> {
        let path = "./src/res/cube_scene.gltf";
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
            }
        }
        instances
    }
}
