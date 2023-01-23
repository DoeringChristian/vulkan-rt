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

// Output Images
layout(set = 1, binding = 0, rgba32f) uniform image2D o_color;

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
#include "integrator/path-restir.glsl"

void main_initial_smapling() {
    render(uvec2(gl_LaunchSizeEXT.xy), uvec2(gl_LaunchIDEXT.xy));
}

void main_spatial_resampling(){
    restir_spatial_resampling(uvec2(gl_LaunchSizeEXT.xy), uvec2(gl_LaunchIDEXT.xy));
}
