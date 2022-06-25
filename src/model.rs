use std::hash::Hash;

use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;
use bytemuck::cast_slice;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub uv01: [f32; 4],
}

#[derive(Component)]
pub struct Vertices {
    pub vertices: Vec<Vertex>,
    pub has_normal: bool,
    pub has_uv0: bool,
    pub has_uv1: bool,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Index(pub u32);

#[derive(Component, Debug)]
pub struct Camera {
    pub up: [f32; 3],
    pub right: [f32; 3],
    pub pos: [f32; 3],
    pub focus: f32,
    pub diameter: f32,
    pub fov: f32,
}

//===================================
// Data that can be used in shaders.
//===================================

///
/// Data relating to an instance used to acces materials etc. in the shader.
///
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslInstanceData {
    pub transform: [[f32; 4]; 4],
    pub mat_index: u32,
    pub indices: u32,
    pub vertices: u32,

    pub normal_uv_mask: u32,
}

///
/// Material to use in the shader.
///
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslMaterial {
    pub albedo: [f32; 4],
    pub mr: [f32; 4],
    pub emission: [f32; 4],
    pub diffuse_tex: u32,
    pub mr_tex: u32,
    pub emission_tex: u32,
    pub normal_tex: u32,
    //pub _pad: [u32; 2],
}

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
}
