use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;
use bytemuck::cast_slice;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Position(pub [f32; 3]);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Index(pub u32);

#[derive(Component)]
pub struct VertexData<T>(pub Vec<T>);

#[derive(Component)]
pub struct MeshId(pub Entity);

#[derive(Component)]
pub struct MaterialId(pub Entity);

#[derive(Component)]
pub struct MeshInstance;

#[derive(Bundle)]
pub struct InstanceBundle {
    pub mesh: MeshId,
    pub material: MaterialId,
    pub transform: Transform,
}

//===================================
// Data that can be used in shaders.
//===================================

///
/// Data relating to an instance used to acces materials etc. in the shader.
///
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslInstanceData {
    pub mat_index: u32,
    pub indices: u32,
    pub positions: u32,
    //pub _pad: [u32; 2],
}

///
/// Material to use in the shader.
///
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslMaterial {
    pub diffuse: [f32; 4],
    pub mra: [f32; 4],
    pub emission: [f32; 4],
}
