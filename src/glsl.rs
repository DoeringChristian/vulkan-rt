use crevice::std140::AsStd140;
use glam::*;

#[derive(AsStd140, Debug)]
pub struct Mesh {
    pub indices: u32,
    pub indices_count: u32,
    pub positions: u32,
    pub normals: u32,
    pub uvs: u32,
}

#[derive(AsStd140, Debug)]
pub struct Instance {
    pub to_world: Mat4,
    pub mesh: u32,
    pub material: u32,
    pub emitter: i32,
}

#[derive(AsStd140, Debug)]
pub struct Emitter {
    pub irradiance: Texture,
    pub instance: u32,
    pub ty: u32,
}

impl Emitter {
    pub fn env(irradiance: Texture) -> Self {
        Self {
            irradiance,
            instance: 0,
            ty: 0,
        }
    }
    pub fn area(irradiance: Texture, instance: u32) -> Self {
        Self {
            irradiance,
            instance,
            ty: 1,
        }
    }
}

#[derive(AsStd140, Debug)]
pub struct Texture {
    pub val: Vec3,
    pub texture: u32,
    pub ty: u32,
}

impl Texture {
    const TY_CONSTANT: u32 = 0;
    const TY_IMAGE: u32 = 1;
    pub fn constant(val: Vec3) -> Self {
        Self {
            ty: Self::TY_CONSTANT,
            val,
            texture: 0,
        }
    }
    pub fn image(texture: u32) -> Self {
        Self {
            ty: Self::TY_IMAGE,
            val: Vec3::ZERO,
            texture,
        }
    }
}

#[derive(AsStd140, Debug)]
pub struct Material {
    pub base_color: Texture,
    pub emission: Texture,
    pub normal: Texture,
    pub metallic_roughness: Texture,
    pub transmission: Texture,
}

#[derive(AsStd140, Debug)]
pub struct Camera {
    pub to_world: Mat4,
    pub to_view: Mat4,
    pub near_clip: f32,
    pub far_clip: f32,
}

impl Camera {
    pub fn perspective(
        to_world: Mat4,
        fov_y: f32,
        aspect_ratio: f32,
        near_clip: f32,
        far_clip: f32,
    ) -> Self {
        let to_view = Mat4::perspective_lh(fov_y, aspect_ratio, near_clip, far_clip);
        let to_view = Mat4::from_translation(vec3(1., 1., 0.)) * to_view;
        let to_view = Mat4::from_scale(vec3(0.5, 0.5, 1.)) * to_view;
        #[cfg(not(target_arch = "spirv"))]
        {
            //println!("{:#?}", to_view);
        }
        Self {
            to_world,
            to_view,
            near_clip,
            far_clip,
            //size: glam::uvec2(width, height),
        }
    }
}

#[derive(AsStd140, Debug)]
pub struct PushConstant {
    pub camera: u32,
    pub max_depth: u32,
    pub rr_depth: u32,
    pub seed: u32,
}
