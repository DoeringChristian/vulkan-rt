#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr, asm_experimental_arch,),
    register_attr(spirv)
)]
#![deny(warnings)]
#![allow(unused, dead_code)]

use common::{Camera, Instance, Material, Payload, Vertex};
use rand::{rand2f, rand3f, randu};
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[allow(unused)]
use spirv_std::RuntimeArray;

#[allow(unused_imports)]
use spirv_std::glam::*;
use spirv_std::{
    num_traits::Float,
    ray_tracing::{AccelerationStructure, RayFlags},
    Image,
};

#[allow(unused_imports)]
use core::arch::asm;

mod common;
#[macro_use]
mod glam_macro;
mod rand;

use glam_macro::*;

pub unsafe fn convert_u_to_ptr<T>(handle: u64) -> *mut T {
    let result: *mut T;
    asm!(
        "{result} = OpConvertUToPtr typeof{result} {handle}",
        handle = in(reg) handle,
        result = out(reg) result,
    );
    result
}

#[spirv(ray_generation)]
pub fn main_rgen(
    #[spirv(ray_payload)] ray: &mut Payload,
    #[spirv(descriptor_set = 0, binding = 0, uniform_constant)] tlas: &AccelerationStructure,
    #[spirv(descriptor_set = 1, binding = 0, uniform_constant)] image: &mut Image!(2D, format = rgba32f, sampled = false, depth = false),
    #[spirv(push_constant)] camera: &Camera,
    #[spirv(launch_size)] launch_size: IVec3,
    #[spirv(launch_id)] launch_id: IVec3,
) {
    #[allow(non_snake_case)]
    let N = camera.fc;
    let mut uv = launch_id.xy().as_vec2();

    let mut seed = N;
    seed = randu(&mut seed) + launch_id.x as u32;
    seed = randu(&mut seed) + launch_id.y as u32;
    seed = randu(&mut seed);

    let roff = rand2f(&mut seed);
    //uv += roff;
    uv /= launch_size.xy().as_vec2();
    uv = (uv * 2. - 1.) * vec2(1., 1.);
    uv *= (camera.fov / 2.).tan();
    //let up = vec3(camera.up[0], camera.up[1], camera.up[2]);
    //let right = vec3(camera.right[0], camera.right[1], camera.right[2]);
    let up = camera.up.xyz();
    let right = camera.right.xyz();
    let forward = up.cross(right).normalize();

    //ray.orig = vec3(camera.pos[0], camera.pos[1], camera.pos[2]);
    ray.orig = camera.pos.xyz();
    ray.dir = (up * uv.x + right * uv.y + forward).normalize();

    ray.color = vec3(0., 0., 0.);
    ray.attenuation = vec3(1., 1., 1.);
    ray.ior = 1.;

    ray.seed = seed;
    ray.depth = 0;
    ray.ray_active = 1;

    let color: Vec4 = image.read(launch_id.xy());
    let mut color = color.xyz();

    for i in 0..camera.depth {
        unsafe {
            tlas.trace_ray(
                RayFlags::OPAQUE,
                0xff,
                0,
                0,
                0,
                ray.orig,
                0.001,
                ray.dir,
                10000.0,
                ray,
            );
        }
    }

    if N == 0 {
        color = ray.color;
    } else {
        let n = N as f32;
        //color = 1. / ((N + 1) as f32) * ray.color + N as f32 / ((N + 1) as f32) * color;
        color = 1. / (n + 1.) * ray.color + n / (n + 1.) * color;
    }

    //color = vec3(0., 1., 0.);
    unsafe {
        image.write(launch_id.xy(), vec4(color.x, color.y, color.z, 1.));
    }
}

#[spirv(closest_hit)]
pub fn main_rchit(
    #[spirv(incoming_ray_payload)] ray: &mut Payload,
    #[spirv(hit_attribute)] hit_co: &mut Vec2,
    #[spirv(descriptor_set = 0, binding = 1, storage_buffer)] instances: &[Instance],
    #[spirv(descriptor_set = 0, binding = 2, storage_buffer)] materials: &[Material],
    #[spirv(instance_custom_index)] index: u32,
    #[spirv(primitive_id)] primitive_id: u32,
) {
    if ray.ray_active == 0 {
        return;
    }

    let inst = instances[index as usize];
    let mat = materials[inst.mat_index as usize];
    let transform = inst.transform;

    let indices: *mut RuntimeArray<u32> = unsafe { convert_u_to_ptr(inst.indices) };
    let vertices: *mut RuntimeArray<Vertex> = unsafe { convert_u_to_ptr(inst.vertices) };

    let i0 = unsafe { (*indices).index(3 * primitive_id as usize + 0) };
    let i1 = unsafe { (*indices).index(3 * primitive_id as usize + 1) };
    let i2 = unsafe { (*indices).index(3 * primitive_id as usize + 2) };

    let vert0 = unsafe { (*vertices).index(*i0 as usize) };
    let vert1 = unsafe { (*vertices).index(*i1 as usize) };
    let vert2 = unsafe { (*vertices).index(*i2 as usize) };

    let mut pos0 = vert0.pos.xyz();
    let mut pos1 = vert1.pos.xyz();
    let mut pos2 = vert2.pos.xyz();

    pos0 = (transform * vec4!(pos0, 1.)).xyz();
    pos1 = (transform * vec4!(pos1, 1.)).xyz();
    pos2 = (transform * vec4!(pos2, 1.)).xyz();

    let barycentric = vec3(1. - hit_co.x - hit_co.y, hit_co.x, hit_co.y);

    let pos = pos0 * barycentric.x + pos1 * barycentric.y + pos2 * barycentric.z;

    let gnorm = (pos1 - pos0).cross(pos2 - pos1).normalize();

    //ray.color = mat.albedo.xyz();
    ray.color = gnorm.xyz();

    //ray.color = vec3(1., 0., 0.);
    ray.ray_active = 0;
}

#[spirv(miss)]
pub fn main_miss(#[spirv(incoming_ray_payload)] ray: &mut Payload) {
    ray.color = vec3(0., 0., 0.);
    ray.ray_active = 0;
}
