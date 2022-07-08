use std140::*;

///
/// Data relating to an instance used to acces materials etc. in the shader.
///
#[repr_std140]
#[derive(Clone, Copy)]
pub struct InstanceData {
    pub trans0: vec4,
    pub trans1: vec4,
    pub trans2: vec4,
    pub trans3: vec4,

    pub mat_index: uint,
    pub mesh_index: uint,
}

///
/// Material to use in the shader.
///
#[repr_std140]
#[derive(Clone, Copy)]
pub struct Material {
    pub albedo: vec4,
    pub emission: vec4,
    pub metallic: float,
    pub roughness: float,
    pub transmission: float,
    pub transmission_roughness: float,
    pub ior: float,
    pub albedo_tex: uint,
    pub mr_tex: uint,
    pub emission_tex: uint,
    pub normal_tex: uint,
}
