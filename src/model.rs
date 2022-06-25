use std::hash::Hash;
use std::sync::Arc;

use crate::{accel::Blas, buffers::TypedBuffer, dense_arena::*};
use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;
use bytemuck::cast_slice;

new_key_type! {
    pub struct TextureKey;
    pub struct MeshKey;
    pub struct BlasKey;
    pub struct InstanceKey;
    pub struct MaterialKey;
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub uv01: [f32; 4],
}

pub struct MeshInstance {
    pub transform: Transform,
    pub mesh: MeshKey,
    pub material: MaterialKey,
}

pub struct Material {
    pub albedo: [f32; 4],
    pub mr: [f32; 4],
    pub emission: [f32; 3],
    pub albedo_tex: Option<TextureKey>,
    pub mr_tex: Option<TextureKey>,
    pub emission_tex: Option<TextureKey>,
    pub normal_tex: Option<TextureKey>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Index(pub u32);

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
