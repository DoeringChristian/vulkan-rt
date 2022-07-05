#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr, asm_experimental_arch,),
    register_attr(spirv)
)]
#![deny(warnings)]
#![allow(unused, dead_code)]

use common::{Camera, HitInfo, Instance, Material, Payload, Vertex};
use rand::{rand2f, rand3f, randf, randu};
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[allow(unused)]
use spirv_std::RuntimeArray;

#[allow(unused_imports)]
use spirv_std::glam::*;
use spirv_std::{
    image::SampledImage,
    num_traits::Float,
    ray_tracing::{AccelerationStructure, RayFlags},
    Image, Sampler,
};

#[allow(unused_imports)]
use core::arch::asm;

mod common;
#[macro_use]
mod glam_macro;
mod bsdf01;
mod rand;

use glam_macro::*;

const INDEX_UNDEF: u32 = 0xffffffff;
const MIN_RR: u32 = 2;
pub unsafe fn convert_u_to_ptr<T>(handle: u64) -> *mut T {
    let result: *mut T;
    asm!(
        "{result} = OpConvertUToPtr typeof{result} {handle}",
        handle = in(reg) handle,
        result = out(reg) result,
    );
    result
}

#[allow(non_snake_case)]
pub fn compute_TBN(duv0: Vec2, duv1: Vec2, dpos0: Vec3, dpos1: Vec3, n: Vec3) -> Mat3 {
    let r = 1. / (duv0.x * duv1.y - duv0.y * duv1.x);
    let t = (dpos0 * duv1.y - dpos1 * duv0.y) * r;
    let b = (dpos1 * duv0.x - dpos0 * duv1.x) * r;
    mat3(t, b, n)
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
    //#[spirv(descriptor_set = 0, binding = 3, uniform)] textures: &Textures,
    #[spirv(descriptor_set = 0, binding = 3)] textures: &RuntimeArray<
        SampledImage<Image!(2d, type = f32, sampled, depth = false)>,
    >,
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
    let norm0 = vert0.normal.xyz();
    let norm1 = vert1.normal.xyz();
    let norm2 = vert2.normal.xyz();

    let norm = if norm0.length_squared() > 0.1
        && norm1.length_squared() > 0.1
        && norm2.length_squared() > 0.1
    {
        let norm0 = Vec3::normalize(norm0);
        let norm1 = Vec3::normalize(norm1);
        let norm2 = Vec3::normalize(norm2);

        let norm = norm0 * barycentric.x + norm1 * barycentric.y + norm2 * barycentric.z;
        (Mat3::from_mat4(transform).transpose().inverse() * norm).normalize()
    } else {
        gnorm
    };

    let prev_orig = ray.orig;
    let prev_dir = ray.dir;

    let wo = (-prev_dir).normalize();
    let dist = prev_orig.distance(pos);

    let mut hit = HitInfo {
        albedo: mat.albedo,
        emission: mat.emission,
        metallic: mat.mr.x,
        roughness: mat.mr.y,
        transmission: mat.transmission,
        ior: mat.ior,

        pos,
        wo,
        gnorm,
        norm,
        dist,
    };

    let uv0 = vert0.uv.xy();
    let uv1 = vert1.uv.xy();
    let uv2 = vert2.uv.xy();
    let uv = uv0 * barycentric.x + uv1 * barycentric.y + uv2 * barycentric.z;

    if mat.diffuse_tex != INDEX_UNDEF {
        let mr: Vec4 = unsafe { textures.index(mat.mr_tex as usize).sample_by_lod(uv, 0.) };
        hit.metallic = mr.z;
        hit.roughness = mr.y;
    }
    if mat.normal_tex != INDEX_UNDEF {
        let tbn = compute_TBN(uv1 - uv0, uv2 - uv0, pos1 - pos0, pos2 - pos0, norm);

        let norm_tex: Vec4 = unsafe {
            textures
                .index(mat.normal_tex as usize)
                .sample_by_lod(uv, 0.)
        };
        let mut norm_tex = norm_tex.xyz();
        norm_tex = vec3(norm_tex.x, 1. - norm_tex.y, norm_tex.z);
        norm_tex = Vec3::normalize(norm_tex * 2. - 1.);
        hit.norm = Vec3::normalize(tbn * norm_tex);
    }

    bsdf01::sample_shader(hit, ray);

    // Russian Roulette
    let mut p_rr = ray
        .attenuation
        .x
        .max(ray.attenuation.y)
        .max(ray.attenuation.z);
    if ray.depth < MIN_RR {
        p_rr = 1.;
    }

    if randf(&mut ray.seed) < p_rr {
        ray.attenuation *= 1. / p_rr;
    } else {
        ray.ray_active = 0;
        return;
    }
}

#[spirv(miss)]
pub fn main_miss(#[spirv(incoming_ray_payload)] ray: &mut Payload) {
    ray.color = vec3(0., 0., 0.);
    ray.ray_active = 0;
}
