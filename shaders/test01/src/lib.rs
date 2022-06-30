#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]
//#![cfg(target_feature = "ext:SPV_KHR_ray_tracing")]
//#![no_std]
//#![cfg(target_feature = "ext:SPV_KHR_ray_tracing")]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
//#![deny(warnings)]
mod common;
use common::*;

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use spirv_std::byte_addressable_buffer::ByteAddressableBuffer;
#[allow(unused_imports)]
use spirv_std::glam::*;
use spirv_std::{
    image::{Image2d, ImageFormat, StorageImage2d},
    ray_tracing::{AccelerationStructure, RayFlags, RayQuery},
    vector::Vector,
    Image, RuntimeArray, Sampler,
};

pub struct Vertices {
    vertices: RuntimeArray<Vertex>,
}

#[spirv(ray_generation)]
#[allow(unused_variables)]
pub fn main_rgen(
    #[spirv(ray_payload)] payload: &mut Payload,
    #[spirv(descriptor_set = 0, binding = 0, uniform_constant)] tlas: &AccelerationStructure,
    #[spirv(descriptor_set = 0, binding = 1, uniform_constant)] image: &mut Image!(2D, format = rgba32f, sampled = false),
    //#[spirv(descriptor_set = 0, binding = 4, storage_buffer)] vertices: &[Vertices],
    #[spirv(push_constant)] camera: &Camera,
    #[spirv(launch_size)] launch_size: IVec3,
    #[spirv(launch_id)] launch_id: IVec3,
) {
    let N = camera.fc;
    let mut uv = launch_id.xy().as_vec2();
    uv /= launch_id.xy().as_vec2();
    uv = (uv * 2. - 1.) * vec2(1., 1.);

    let up = camera.up.xyz();
    let right = camera.right.xyz();
    let forward = up.cross(right).normalize();

    payload.orig = camera.pos.xyz();
    payload.dir = Vec3::normalize(up * uv.x + right * uv.y + forward);

    payload.color = vec3(0., 0., 0.);
    payload.attenuation = vec3(0., 0., 0.);
    payload.ior = 1.;

    payload.depth = 0;
    payload.ray_active = 1;

    let color: Vec4 = image.read(launch_id.xy());
    let mut color = color.xyz();

    //for i in 0..camera.depth {
    for i in 0..1 {
        unsafe {
            tlas.trace_ray(
                RayFlags::OPAQUE,
                0xff,
                0,
                0,
                0,
                payload.orig,
                0.001,
                payload.dir,
                10000.0,
                payload,
            );
        }
    }

    if N == 0 {
        color = payload.color;
    } else {
        color = 1. / ((N + 1) as f32) * payload.color + N as f32 / ((N + 1) as f32) * color;
    }
    unsafe {
        image.write(launch_id.xy(), Vec4::from((color, 1.)));
    }
}

#[spirv(closest_hit)]
#[allow(unused_variables)]
pub fn main_rchit(
    #[spirv(incoming_ray_payload)] ray: &mut Payload,
    #[spirv(hit_attribute)] hit_co: &mut Vec2,
) {
    ray.color = vec3(1., 0., 0.);
}

#[spirv(miss)]
pub fn main_miss(#[spirv(incoming_ray_payload)] ray: &mut Payload) {
    ray.color = vec3(0., 0., 0.);
}
