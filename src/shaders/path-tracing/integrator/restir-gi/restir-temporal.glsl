#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "common.glsl"
#include "scene-bindings.glsl"
#include "restir-pushconstant.glsl"

// Ray Tracing Bindings
layout(location = 0) rayPayloadEXT Payload payload;
layout(location = 1) rayPayloadEXT bool shadow_payload;

layout(std140, set = 1, binding = 0) buffer InitialSamples{
    RestirSample initial_samples[];
};
layout(std140, set = 1, binding = 1) buffer TemporalReservoir{
    RestirReservoir temporal_reservoir[];
};
layout(std140, set = 1, binding = 2) buffer SpatialReservoir{
    RestirReservoir spatial_reservoir[];
};

#include "trace.glsl"

#include "sampler/independent.glsl"
#include "bsdf/diffuse.glsl"
#include "camera.glsl"
#include "emitter.glsl"

#include "restir-path.glsl"
#include "restir-reservoir.glsl"

#define M_MAX 30

uint pixel_idx = (gl_LaunchIDEXT.y * gl_LaunchSizeEXT.x + gl_LaunchIDEXT.x);

float p_hat(const vec3 f){
    return length(f);
}

void main(){
    const vec2 pos = vec2(gl_LaunchIDEXT.xy);

    SampleGenerator sample_generator = sample_generator(push_constant.seed, pixel_idx); // TODO: maybe init from sample

    RestirSample S = initial_samples[pixel_idx]; // l.2

    RestirReservoir R = temporal_reservoir[pixel_idx]; // l.3

    if (length(S.n_s) == 0){
        init(R);
    }
    if (push_constant.do_spatiotemporal == 0){
        init(R);
    }

    float w = p_hat(S.L_o)/S.p_q; // l.4
    update(R, S, w, next_1d(sample_generator)); // l.5
    float phat = p_hat(R.z.L_o);
    R.W = phat == 0 ? 0 : R.w / (R.M * phat); // l.6
    
    temporal_reservoir[pixel_idx] = R; // l.7

    // RestirReservoir R_new;
    // R_new.w = 0;
    // R_new.W = 0;
    // R_new.M = 0;
    // float phat = p_hat(S.L_o);
    // float w = phat / S.p_q;
    // update(R_new, S, w, next_1d(sample_generator));
    // R_new.W = phat == 0 ? 0 : R_new.w / (R_new.M * phat);

    // RestirReservoir R_t;
    // R_t.w = 0;
    // R_t.W = 0;
    // R_t.M = 0;
    // update(R_t, R_new.z, R_new.M * R_new.W * phat, next_1d(sample_generator));
    // update(R_t, R.z, R.M * R.W * p_hat(R.z.L_o), next_1d(sample_generator));
    // uint mval = R.M;
    // float new_phat = p_hat(R_t.z.L_o);
    // if (new_phat > 0){
    //     mval ++;
    // }
    // R_t.M = min(R.M + 1, M_MAX);
    // R_t.W = new_phat * mval == 0 ? 0 : R_t.w / (mval * new_phat);
    
    // temporal_reservoir[pixel_idx] = R_s;
}
