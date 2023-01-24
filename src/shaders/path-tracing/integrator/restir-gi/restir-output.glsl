#version 460
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "common.glsl"
#include "push-constant.glsl"

layout(set = 2, binding = 0) buffer InitialSamples{
    RestirSample initial_samples[];
};
layout(set = 2, binding = 1) buffer TemporalReservoir{
    RestirReservoir temporal_reservoir[];
};
layout(set = 2, binding = 2) buffer Spatialreservoir{
    RestirReservoir spatial_reservoir[];
};

#include "sampler/independent.glsl"

#include "restir-reservoir.glsl"

layout(set = 2, binding = 0, rgba32f) uniform image2D o_color;

uint pixel_idx = (gl_NumWorkGroups.y * gl_GlobalInvocationID.x + gl_GlobalInvocationID.x);

void main(){
    ivec2 coords = ivec2(gl_GlobalInvocationID.xy);

    vec3 color = vec3(0);

    RestirReservoir R = spatial_reservoir[pixel_idx];
    if (R.W > 0){
        RestirSample S = R.z;
        color += S.f * S.L_o * R.W;
    }
    imageStore(o_color, ivec2(coords), vec4(color, 1.));
}
