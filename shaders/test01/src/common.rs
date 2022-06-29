use spirv_std::{glam::*, macros::spirv};

#[repr(C)]
pub struct Payload {
    pub orig: Vec3,
    pub dir: Vec3,

    pub color: Vec3,
    pub attenuation: Vec3,
    pub ior: f32,

    pub depth: i32,
    pub ray_active: i32,
}

#[repr(C)]
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
