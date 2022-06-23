use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;
use bytemuck::cast_slice;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Position(pub [f32; 3]);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Index(pub u32);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Normal(pub [f32; 3]);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Tangent(pub [f32; 4]);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexCoord(pub [f32; 2]);

#[derive(Component)]
pub struct TexCoords(pub Vec<VertexData<TexCoord>>);

#[derive(Component)]
pub struct VertexData<T>(pub Vec<T>);

#[derive(Component)]
pub struct Texture(pub image::DynamicImage);

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

pub struct TextureId {
    pub texture: Entity,
    pub coords: u32,
}

#[derive(Component)]
pub struct Material {
    pub albedo: [f32; 4],
    pub mr: [f32; 4],
    pub emission: [f32; 3],
    pub albedo_tex: Option<TextureId>,
    pub mr_tex: Option<TextureId>,
    pub emission_tex: Option<TextureId>,
    pub normal_tex: Option<TextureId>,
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
    pub positions: u32,
    pub normals: u32,
    pub tex_coords: u32,
    pub tex_coords_num: u32,
    pub _pad: [u32; 2],
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
    pub diffuse_texco: u32,
    pub mr_tex: u32,
    pub mr_texco: u32,
    pub emission_tex: u32,
    pub emission_texco: u32,
    pub normal_tex: u32,
    pub normal_texco: u32,
    //pub _pad: [u32; 2],
}
