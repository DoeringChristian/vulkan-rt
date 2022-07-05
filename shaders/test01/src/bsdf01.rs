use crate::{
    common::*,
    rand::{cosine_hemisphere, rand2f, randf},
};
use core::f32::consts::PI;
use spirv_std::{glam::*, num_traits::Float};

pub fn distribution_ggx(n: Vec3, h: Vec3, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = n.dot(h).max(0.);
    let n_dot_h_2 = n_dot_h * n_dot_h;

    let num = a2;
    let denom = (n_dot_h_2 * (a2 - 1.) + 1.);
    let denom = PI * denom * denom;

    num / denom
}

pub fn sample_distribution_ggx(roughness: f32, n: Vec3, seed: &mut u32) -> Vec3 {
    let a = roughness * roughness;
    let a2 = a * a;

    let e = rand2f(seed);

    let theta = (1. - e.x) / ((a2 - 1.) * e.x + 1.);
    let theta = theta.sqrt().acos();
    let phi = 2. * PI * e.y;

    let m = vec3(
        phi.cos() * theta.sin(),
        phi.sin() * theta.sin(),
        theta.cos(),
    );

    allign_hemisphere(m, n)
}

pub fn sample_distribution_beckmann(roughness: f32, n: Vec3, seed: &mut u32) -> Vec3 {
    todo!()
}

pub fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.);
    let k = (r * r) / 8.;

    let num = n_dot_v;
    let denom = n_dot_v * (1. - k) + k;

    num / denom
}

pub fn geometry_smith(n: Vec3, v: Vec3, l: Vec3, roughness: f32) -> f32 {
    let n_dot_v = n.dot(v).max(0.);
    let n_dot_l = n.dot(l).max(0.);
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);

    ggx1 * ggx2
}

pub fn fresnel_schlick(cos_theta: f32, r0: f32) -> f32 {
    let x = 1. - cos_theta;
    let x2 = x * x;
    r0 + (1. - r0) * x2 * x2 * x
}
pub fn fresnel_schlick3(cos_theta: f32, r0: Vec3) -> Vec3 {
    let x = 1. - cos_theta;
    let x2 = x * x;
    r0 + (1. - r0) * x2 * x2 * x
}
pub fn fresnel_schlick_n(mut cos_theta: f32, n1: f32, n2: f32) -> f32 {
    let mut r0 = (n1 - n2) / (n1 + n2);
    r0 *= r0;
    if n1 > n2 {
        let n = n1 / n2;
        let sin2_theta = n * n * (1. - cos_theta * cos_theta);
        if sin2_theta > 1. {
            return 1.;
        }
        cos_theta = (1. - sin2_theta).sqrt();
    }
    fresnel_schlick(cos_theta, r0)
}

pub fn sample_diffuse(hit: &HitInfo, ray: &mut Payload) {
    let wi = allign_hemisphere(cosine_hemisphere(&mut ray.seed), hit.norm);

    let fr = hit.albedo.xyz() / PI;

    ray.attenuation *= fr * (2. * PI);
    ray.dir = wi;
}

pub fn sample_refraction(hit: &HitInfo, ray: &mut Payload, m: Vec3, n1: f32, n2: f32) {
    let wi = refract(-hit.wo, m, n1 / n2);
    let wi_dot_n = m.dot(-wi).max(0.);
    let n_dot_v = hit.norm.dot(wi).max(0.);
    let g = geometry_schlick_ggx(n_dot_v, hit.roughness);

    let fr = vec3(1., 1., 1.);

    ray.attenuation *= fr * wi_dot_n * (2. * PI);
    ray.dir = wi;
}

pub fn sample_specular_refl(hit: &HitInfo, ray: &mut Payload, m: Vec3) {
    let wi = reflect(-hit.wo, m);
    let wi_dot_n = m.dot(wi).max(0.);
    let g = geometry_smith(hit.norm, hit.wo, wi, hit.roughness);

    let numerator = g * vec3(1., 1., 1.);
    let denominator = 4. * m.dot(hit.wo).max(0.) * m.dot(wi).max(0.) + 0.0001;
    let specular = numerator / denominator;
    let fr = specular;

    ray.attenuation *= fr * wi_dot_n * (2. * PI);
    ray.dir = wi;
}

pub fn sample_specular(hit: &HitInfo, ray: &mut Payload, m: Vec3) {
    sample_specular_refl(hit, ray, m);
}

pub fn sample_dielectric(hit: &HitInfo, ray: &mut Payload, m: Vec3, n1: f32, n2: f32) {
    let f0_sqrt = (n1 - n2) / (n1 + n2);
    let f0 = f0_sqrt * f0_sqrt;

    let k_s = fresnel_schlick_n(m.dot(hit.wo), n1, n2);
    let k_d = 1. - k_s;

    if randf(&mut ray.seed) < k_s {
        sample_specular(hit, ray, m);
    } else {
        if randf(&mut ray.seed) >= hit.transmission {
            sample_diffuse(hit, ray);
        } else {
            sample_refraction(hit, ray, m, n1, n2);
        }
    }
}

pub fn sample_metallic(hit: &HitInfo, ray: &mut Payload, m: Vec3, n1: f32, n2: f32) {
    let f0 = hit.albedo.xyz();
    let f = fresnel_schlick3(m.dot(hit.wo).clamp(0., 1.), f0);

    sample_specular(hit, ray, m);
    ray.attenuation *= f;
}

pub fn sample_shader(mut hit: HitInfo, ray: &mut Payload) {
    ray.orig = hit.pos;
    ray.color += ray.attenuation * hit.emission.xyz();

    let n1;
    let n2;
    if hit.gnorm.dot(hit.wo) < 0. {
        hit.norm = -hit.norm;
        n1 = hit.ior;
        n2 = 1.;
    } else {
        n1 = 1.;
        n2 = hit.ior;
    }

    let m = sample_distribution_ggx(hit.roughness, hit.norm, &mut ray.seed);

    if randf(&mut ray.seed) < hit.metallic {
        sample_dielectric(&hit, ray, m, n1, n2);
    } else {
        sample_metallic(&hit, ray, m, n1, n2);
    }
}
