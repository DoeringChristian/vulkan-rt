use std140::*;

/// A column vector of 2 [float] values.
///
/// # Example
///
/// ```
/// let value = std140::vec2(0.0, 1.0);
/// ```
#[allow(non_camel_case_types)]
#[repr(C, align(8))]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct uint64_t(pub u64);

unsafe impl ReprStd140 for uint64_t {}
unsafe impl Std140ArrayElement for uint64_t {}

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
    pub indices: uint64_t,
    pub vertices: uint64_t,
}

///
/// Material to use in the shader.
///
#[repr_std140]
#[derive(Clone, Copy)]
pub struct MaterialData {
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

    pub med: MediumData,
}

#[repr_std140]
#[derive(Clone, Copy)]
pub struct LightData {
    pub emission: vec4,
    pub position: vec4,
    pub radius: float,
    pub light_type: uint,
}

impl LightData {
    pub const TY_POINT: uint = uint(0);
}

#[repr_std140]
#[derive(Clone, Copy)]
pub struct MediumData {
    pub color: vec4,
    pub anisotropic: float,
    pub density: float,
}
