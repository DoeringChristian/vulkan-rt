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

layout(set = 1, binding = 0) buffer InitialSamples{
    RestirSample initial_samples[];
};
layout(set = 1, binding = 1) buffer TemporalReservoir{
    RestirReservoir temporal_reservoir[];
};
layout(set = 1, binding = 2) buffer Spatialreservoir{
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

float p_hat(const vec3 f){
    return length(f);
}

void main(){
    const vec2 pos = vec2(gl_LaunchIDEXT.xy);

    SampleGenerator sample_generator = sample_generator(push_constant.seed, pixel_idx); // TODO: maybe init from sample

    RestirSample S = initial_samples[pixel_idx];

    RestirReservoir R = temporal_reservoir[pixel_idx];

    if (length(S.n_s) == 0){
        init(R);
        init(spatial_reservoir[pixel_idx]);
    }
    
    float phat = p_hat(S.L_o);
    float w = phat / S.p_q;
    update(R, S, w, next_1d(sample_generator));
    R.W = phat == 0 ? 0 : R.w / (R.M * phat);

    temporal_reservoir[pixel_idx] = R;
}
