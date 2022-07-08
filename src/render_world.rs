use crate::{
    buffers::TypedBuffer,
    dense_arena::DenseArena,
    model::{
        GlslCamera, Index, InstanceKey, Light, LightKey, Material, MaterialKey, Mesh, MeshInstance,
        MeshKey, ShaderGroup, ShaderGroupKey, ShaderKey, TextureKey, Vertex,
    },
};
use glam::*;
use screen_13::prelude::*;
use screen_13_fx::ImageLoader;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

#[derive(Default)]
pub struct RenderWorld {
    pub meshes: DenseArena<MeshKey, Mesh>,
    pub textures: DenseArena<TextureKey, Arc<Image>>,
    pub materials: DenseArena<MaterialKey, Material>,
    pub instances: DenseArena<InstanceKey, MeshInstance>,
    pub shaders: DenseArena<ShaderKey, Shader>,
    pub shader_groups: DenseArena<ShaderGroupKey, ShaderGroup>,
    pub lights: DenseArena<LightKey, Light>,
    pub camera: GlslCamera,
    //pub events: HashSet<RenderWorldEvent>,
}

impl RenderWorld {
    pub fn set_camera(&mut self, camera: GlslCamera) {
        //self.set_event(RenderWorldEvent::CameraChanged);
        self.camera = camera;
    }
    pub fn get_camera(&self) -> GlslCamera {
        self.camera
    }
    pub fn insert_shader(&mut self, shader: Shader) -> ShaderKey {
        //self.set_event(RenderWorldEvent::ShadersResized);
        self.shaders.insert(shader)
    }
    pub fn insert_shader_group(&mut self, group: ShaderGroup) -> ShaderGroupKey {
        //self.set_event(RenderWorldEvent::ShaderGroupsResized);
        self.shader_groups.insert(group)
    }
    pub fn insert_texture(
        &mut self,
        device: &Arc<Device>,
        img: &image::DynamicImage,
    ) -> TextureKey {
        //self.set_event(RenderWorldEvent::TexturesResized);
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
    pub fn insert_light(&mut self, light: Light) -> LightKey {
        self.lights.insert(light)
    }
    pub fn insert_material(&mut self, material: Material) -> MaterialKey {
        //self.set_event(RenderWorldEvent::MaterialsResized);
        self.materials.insert(material)
    }
    pub fn insert_instance(&mut self, instance: MeshInstance) -> InstanceKey {
        //self.set_event(RenderWorldEvent::InstancesResized);
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
        //self.set_event(RenderWorldEvent::MeshResized(key));
        key
    }
}
