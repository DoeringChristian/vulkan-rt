#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "restir-common.glsl"

uint pixel_idx = (gl_LaunchIDEXT.y * gl_LaunchSizeEXT.x + gl_LaunchIDEXT.x);

void main(){
    const vec2 pos = vec2(gl_LaunchIDEXT.xy);

    Sampler sampler = sampler(push_constant.seed, pixel_idx);

    vec2 sample_pos = vec2(pixel) + next_2d();
    vec2 adjusted_pos = sample_pos / vec2(gl_LaunchSizeEXT.xy);

    Ray ray = sample_ray(adjusted_pos);


    RestirSample S;

    SurfaceInteraction si = ray_intersect(ray); // Trace to find x_v

    S.x_v = si.p;
    S.n_v = si.n;

    BSDFSample bs;
    vec3 bsdf_value;
    sample_bsdf(si, next_2d(), bs, bsdf_value);

    S.p_q = bs.pdf;
    S.f = bsdf_value;

    ray = spawn_ray(to_world(si, bs.wo));

    si = ray_intersect(ray); // Trace to find x_s

    S.x_s = si.p;
    S.n_s = si.n;

    vec3 Lo = sample_outgoing(si_s);

    S.Lo = Lo;

    initial_samples[pixel_idx] = S;
}
