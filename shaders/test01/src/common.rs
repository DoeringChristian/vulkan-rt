use core::f32;

use spirv_std::{glam::*, num_traits::Float};

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Material {
    pub albedo: Vec4,
    pub mr: Vec4,
    pub emission: Vec4,
    pub transmission: f32,
    pub transmission_roughness: f32,
    pub ior: f32,
    pub _pack: u32,
    pub diffuse_tex: u32,
    pub mr_tex: u32,
    pub emission_tex: u32,
    pub normal_tex: u32,
    //pub _pad: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Camera {
    pub up: Vec4,
    pub right: Vec4,
    pub pos: Vec4,
    pub focus: f32,
    pub diameter: f32,
    pub fov: f32,
    pub fc: u32,
    pub depth: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PushConstant {
    pub camera: Camera,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Instance {
    //pub transform: [[f32; 4]; 4],
    pub transform: Mat4,
    pub mat_index: u32,
    pub normal_uv_mask: u32,
    pub vertices: u64,
    pub indices: u64,
    pub _pad0: u32,
    pub _pad1: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: Vec4,
    pub normal: Vec4,
    pub uv: Vec2,
    pub uv2: Vec2,
}

#[repr(C)]
pub struct HitInfo {
    pub albedo: Vec4,
    pub emission: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub transmission: f32,
    pub ior: f32,

    pub pos: Vec3,
    pub wo: Vec3,

    pub gnorm: Vec3,
    pub norm: Vec3,
    pub dist: f32,
}

#[repr(C)]
pub struct Payload {
    pub orig: Vec3,
    pub dir: Vec3,

    pub color: Vec3,
    pub attenuation: Vec3,
    pub ior: f32,

    pub seed: u32,
    pub depth: u32,
    pub active: u32,
}

pub fn allign_hemisphere(hemisphere: Vec3, up: Vec3) -> Vec3 {
    let right = Vec3::normalize(Vec3::cross(up, vec3(0.0072, 1., 0.0034)));
    let forward = right.cross(up);

    hemisphere.x * forward + hemisphere.y * right + hemisphere.z * up
}

pub fn refract(incident: Vec3, normal: Vec3, n: f32) -> Vec3 {
    let cos_i = -normal.dot(incident);
    let sin2_t = n * n * (1. - cos_i * cos_i);
    if sin2_t > 1. {
        return Vec3::ZERO;
    }
    let cos_t = (1. - sin2_t).sqrt();
    n * incident + (n * cos_i - cos_t) * normal
}
pub fn reflect(incident: Vec3, normal: Vec3) -> Vec3 {
    incident - 2. * normal.dot(incident) * normal
}

pub trait Mix<F> {
    fn mix(self, other: Self, factor: F) -> Self;
}

impl Mix<f32> for f32 {
    fn mix(self, other: Self, factor: f32) -> Self {
        other * factor + self * (1. - factor)
    }
}
impl Mix<f32> for Vec2 {
    fn mix(self, other: Self, factor: f32) -> Self {
        other * factor + self * (1. - factor)
    }
}
impl Mix<f32> for Vec3 {
    fn mix(self, other: Self, factor: f32) -> Self {
        other * factor + self * (1. - factor)
    }
}
impl Mix<f32> for Vec4 {
    fn mix(self, other: Self, factor: f32) -> Self {
        other * factor + self * (1. - factor)
    }
}
