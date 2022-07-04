use spirv_std::glam::*;

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Material {
    pub albedo: [f32; 4],
    pub mr: [f32; 4],
    pub emission: [f32; 4],
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
    pub ray_active: u32,
}
