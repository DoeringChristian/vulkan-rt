use crate::glsl;
use std::hash::Hash;
use std::sync::Arc;

use crate::{buffers::TypedBuffer, dense_arena::*};
use glam::*;

new_key_type! {
    pub struct TextureKey;
    pub struct MeshKey;
    pub struct BlasKey;
    pub struct InstanceKey;
    pub struct MaterialKey;
    pub struct ShaderKey;
    pub struct ShaderGroupKey;
    pub struct ShaderBindingKeys;
}

pub struct Mesh {
    pub indices: Arc<TypedBuffer<glsl::Index>>,
    pub vertices: Arc<TypedBuffer<glsl::Vertex>>,
}

#[derive(Clone)]
pub struct MeshInstance {
    pub transform: Mat4,
    pub mesh: MeshKey,
    pub material: MaterialKey,
    pub shader_groups: Vec<ShaderGroupKey>,
}

pub struct Material {
    pub albedo: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub emission: Vec3,
    pub transmission: f32,
    pub ior: f32,
    pub transmission_roughness: f32,
    pub albedo_tex: Option<TextureKey>,
    pub mr_tex: Option<TextureKey>,
    pub emission_tex: Option<TextureKey>,
    pub normal_tex: Option<TextureKey>,
    pub transmission_tex: Option<TextureKey>,
}

#[derive(Clone, Copy)]
pub struct Camera {
    pub up: Vec3,
    pub right: Vec3,
    pub pos: Vec3,
    pub focus: f32,
    pub diameter: f32,
    pub fov: f32,
    pub depth: u32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            up: vec3(0., 0., 1.),
            right: vec3(0., 1., 0.),
            pos: vec3(1., 0., 0.),
            focus: 1.,
            diameter: 0.1,
            fov: 1.,
            depth: 16,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslRef(pub u32);

#[allow(unused)]
impl GlslRef {
    const REF_UNDEF: u32 = 0xffffffff;
    pub fn new(index: u32) -> Self {
        if index == Self::REF_UNDEF {
            panic!("This is not a valid index");
        }
        Self(index)
    }
    pub fn none() -> Self {
        Self(Self::REF_UNDEF)
    }
}

#[allow(unused)]
pub enum ShaderGroup {
    General {
        general: ShaderKey,
    },
    Procedural {
        intersection: ShaderKey,
        closest_hit: Option<ShaderKey>,
        any_hit: Option<ShaderKey>,
    },
    Triangle {
        closest_hit: ShaderKey,
        any_hit: Option<ShaderKey>,
    },
}

/*
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstant {
    pub camera: GlslCamera,
}
*/
