use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;
use bytemuck::cast_slice;

pub trait AsSlice<T> {
    fn as_slice(&self) -> &[T];
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Position(pub [f32; 3]);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Index(pub u32);

#[derive(Component)]
pub struct VertexData<T>(pub Vec<T>);

impl AsSlice<[f32; 3]> for VertexData<Position> {
    fn as_slice(&self) -> &[[f32; 3]] {
        cast_slice(&self.0)
    }
}

impl AsSlice<u32> for VertexData<Index> {
    fn as_slice(&self) -> &[u32] {
        cast_slice(&self.0)
    }
}

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
