use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Indices(pub Vec<u32>);

#[derive(Component)]
pub struct Positions(pub Vec<f32>);
