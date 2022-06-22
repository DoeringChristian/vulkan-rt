use bevy_ecs::prelude::*;
use bevy_transform::prelude::*;

#[derive(Component)]
pub struct Indices(pub Vec<u32>);

#[derive(Component)]
pub struct Positions(pub Vec<f32>);

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
