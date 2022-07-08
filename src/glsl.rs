//use crevice::glsl::*;
use encase::*;
use glam::*;

use crate::accel::BlasPosition;

// Holds all types that can be uploaded to shader

#[repr(C)]
#[derive(ShaderType, Clone, Copy)]
pub struct InstanceData {
    pub trans0: Vec4,
    pub trans1: Vec4,
    pub trans2: Vec4,
    pub trans3: Vec4,

    pub mat_index: u32,
    pub mesh_index: u32,
    pub _pad: [u32; 2],
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy)]
pub struct Material {
    pub albedo: Vec4,
    pub emission: Vec4,

    pub metallic: f32,
    pub roughness: f32,
    pub transmission: f32,
    pub transmission_roughness: f32,

    pub ior: f32,

    pub albedo_tex: u32,
    pub mr_tex: u32,
    pub emission_tex: u32,
    pub normal_tex: u32,

    pub _pad: [u32; 1],
}

#[repr(C)]
#[derive(ShaderType, Clone, Copy)]
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

impl Default for Camera {
    fn default() -> Self {
        Self {
            up: vec4(0., 0., 1., 1.),
            right: vec4(0., 1., 0., 1.),
            pos: vec4(1., 0., 0., 1.),
            focus: 1.,
            diameter: 0.1,
            fov: 1.,
            fc: 0,
            depth: 16,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub uv: [f32; 4],
}

impl BlasPosition for Vertex {
    fn vertex_format() -> screen_13::prelude_arc::vk::Format {
        screen_13::prelude::vk::Format::R32G32B32_SFLOAT
    }
    fn vertex_stride() -> screen_13::prelude::vk::DeviceSize {
        std::mem::size_of::<Vertex>() as _
    }
}

//#[repr_std140]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Index(pub u32);
