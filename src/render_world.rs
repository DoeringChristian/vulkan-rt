use crate::{
    dense_arena::DenseArena,
    model::{
        InstanceKey, Material, MaterialKey, Mesh, MeshInstance, MeshKey, ShaderGroup,
        ShaderGroupKey, ShaderKey, TextureKey,
    },
};
use screen_13::prelude::{Image, Shader};
use std::sync::Arc;

pub struct RenderWorld {
    pub meshes: DenseArena<MeshKey, Mesh>,
    pub textures: DenseArena<TextureKey, Arc<Image>>,
    pub materials: DenseArena<MaterialKey, Material>,
    pub instances: DenseArena<InstanceKey, MeshInstance>,
    pub shaders: DenseArena<ShaderKey, Shader>,
    pub shader_groups: DenseArena<ShaderGroupKey, ShaderGroup>,
}

