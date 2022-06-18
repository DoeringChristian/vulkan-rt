use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Model {
    pub indices: Vec<u32>,
    pub positions: Vec<f32>,
    //pub uvs: Vec<f32>,
}
