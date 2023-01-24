#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "common.glsl"
#include "scene-bindings.glsl"

// Ray Tracing Bindings
layout(location = 0) rayPayloadEXT Payload payload;
layout(location = 1) rayPayloadEXT bool shadow_payload;

layout(set = 2, binding = 0) buffer InitialSamples{
    RestirSample initial_samples[];
};
layout(set = 2, binding = 1) buffer TemporalReservoir{
    RestirReservoir temporal_reservoir[];
};
layout(set = 2, binding = 2) buffer Spatialreservoir{
    RestirReservoir spatial_reservoir[];
};

#include "trace.glsl"

#include "sampler/independent.glsl"
#include "bsdf/diffuse.glsl"
#include "camera.glsl"
#include "emitter.glsl"

#include "restir-path.glsl"
#include "restir-reservoir.glsl"

uint pixel_idx = (gl_LaunchIDEXT.y * gl_LaunchSizeEXT.x + gl_LaunchIDEXT.x);
vec2 pixel_pos = vec2(gl_LaunchIDEXT.xy);

void main(){

    SampleGenerator sample_generator = sample_generator(push_constant.seed, pixel_idx);

    vec2 sample_pos = vec2(pixel_pos) + next_2d(sample_generator);
    vec2 adjusted_pos = sample_pos / vec2(gl_LaunchSizeEXT.xy);

    Ray ray = sample_ray(adjusted_pos);


    RestirSample S;

    SurfaceInteraction si = ray_intersect(ray); // Trace to find x_v

    S.x_v = si.p;
    S.n_v = si.n;

    BSDFSample bs;
    vec3 bsdf_value;
    sample_bsdf(si, next_1d(sample_generator), next_2d(sample_generator), bs, bsdf_value);

    S.p_q = bs.pdf;
    S.f = bsdf_value;

    ray = spawn_ray(si, to_world(si, bs.wo));

    si = ray_intersect(ray); // Trace to find x_s

    S.x_s = si.p;
    S.n_s = si.n;

    vec3 Lo = sample_outgoing(si, sample_generator);

    S.L_o = Lo;

    initial_samples[pixel_idx] = S;
}
