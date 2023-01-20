
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "common.glsl"
#include "rand.glsl"

layout(location = 0) rayPayloadInEXT Payload payload;

void main() {
    payload.valid = 0;
}
