
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "common.glsl"
#include "rand.glsl"

layout(location = 0) rayPayloadInEXT Payload payload;

void main() {
    //payload.radiance += vec3(0.) * payload.throughput;
    payload.valid = 0;
    //payload.ray_active = 0;
    //payload.color *= payload.dir * 100.;
}
