use std::hash::Hash;
use std::sync::Arc;

use crate::{buffers::TypedBuffer, dense_arena::*, glsl};
use glam::*;

new_key_type! {
    pub struct TextureKey;
    pub struct MeshKey;
    pub struct BlasKey;
    pub struct InstanceKey;
    pub struct MaterialKey;
    pub struct ShaderKey;
    pub struct ShaderGroupKey;
    pub struct LightKey;
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub uv01: [f32; 4],
}

pub struct Mesh {
    pub indices: Arc<TypedBuffer<Index>>,
    pub vertices: Arc<TypedBuffer<Vertex>>,
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

    pub medium: Medium,
}

pub struct Medium {
    pub color: Vec4,
    pub anisotropic: f32,
    pub density: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Index(pub u32);

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

#[derive(Clone, Copy)]
pub enum Light {
    Point {
        position: Vec3,
        emission: Vec3,
        radius: f32,
        strength: f32,
    },
}

impl Default for Light {
    fn default() -> Self {
        Self::Point {
            position: Vec3::ZERO,
            emission: Vec3::ZERO,
            radius: 0.01,
            strength: 0.,
        }
    }
}

impl From<Light> for glsl::LightData {
    fn from(src: Light) -> Self {
        match src {
            Light::Point {
                position,
                emission,
                radius,
                strength,
            } => Self {
                emission: std140::vec4(emission.x, emission.y, emission.z, strength),
                position: std140::vec4(position.x, position.y, position.z, 1.),
                radius: std140::float(radius),
                light_type: glsl::LightData::TY_POINT,
            },
        }
    }
}

//===================================
// Data that can be used in shaders.
//===================================

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslCamera {
    pub up: [f32; 4],
    pub right: [f32; 4],
    pub pos: [f32; 4],
    pub focus: f32,
    pub diameter: f32,
    pub fov: f32,
    pub fc: u32,
    pub depth: u32,
}
impl Default for GlslCamera {
    fn default() -> Self {
        Self {
            up: [0., 0., 1., 1.],
            right: [0., 1., 0., 1.],
            pos: [1., 0., 0., 1.],
            focus: 1.,
            diameter: 0.1,
            fov: 1.,
            fc: 0,
            depth: 16,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PushConstant {
    pub camera: GlslCamera,
}
