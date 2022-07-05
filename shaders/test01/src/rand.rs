use core::f32::consts::PI;
use spirv_std::{glam::*, num_traits::Float};

pub fn uint_to_unit_float(mut u: u32) -> f32 {
    const MANTISSA_MASK: u32 = 0x007FFFFFu32;
    const ONE: u32 = 0x3F800000u32;
    u &= MANTISSA_MASK;
    u |= ONE;
    let r2 = f32::from_bits(u);
    return r2 - 1.;
}

/*
 * Pcg Hashing algorithm adapted from https://www.shadertoy.com/view/XlGcRh.
 *
 * https://www.pcg-random.org/
*/
pub fn pcg(v: u32) -> u32 {
    let state = v * 747796405u32 + 2891336453u32;
    let word = ((state >> ((state >> 28u32) + 4u32)) ^ state) * 277803737u32;
    (word >> 22u32) ^ word
}

pub fn randf(seed: &mut u32) -> f32 {
    *seed = pcg(*seed);
    uint_to_unit_float(*seed)
}
pub fn randu(seed: &mut u32) -> u32 {
    *seed = pcg(*seed);
    *seed
}

pub fn rand2f(seed: &mut u32) -> Vec2 {
    let x = randf(seed);
    let y = randf(seed);
    vec2(x, y)
}
pub fn rand3f(seed: &mut u32) -> Vec3 {
    let x = randf(seed);
    let y = randf(seed);
    let z = randf(seed);
    vec3(x, y, z)
}

pub fn cosine_hemisphere(seed: &mut u32) -> Vec3 {
    let r = randf(seed).sqrt();
    let phi = randf(seed) * 2. * PI;

    let x = r * phi.cos();
    let y = r * phi.sin();

    vec3(x, y, (1. - x * x - y * y).sqrt())
}

pub fn uniform_hemisphere(seed: &mut u32) -> Vec3 {
    let uv = rand2f(seed);
    let theta = (1. - uv.x).acos();
    let phi = 2. * PI * uv.y;
    vec3(
        phi.cos() * theta.sin(),
        phi.sin() * theta.sin(),
        theta.cos(),
    )
}
