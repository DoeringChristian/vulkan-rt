
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "rand.glsl"
#include "common.glsl"


hitAttributeEXT vec2 hit_co;

layout(location = 0) rayPayloadInEXT SurfaceInteraction si;
layout(location = 1) rayPayloadEXT bool isShadow;

layout(set = 0, binding = 0) uniform accelerationStructureEXT accel;

void main() {
    si.barycentric = vec3(1. - hit_co.x - hit_co.y, hit_co.x, hit_co.y);
    si.instance = gl_InstanceID;
    si.primitive = gl_PrimitiveID;
    si.valid = 1;
}
