#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "util/rand.glsl"
#include "bindings.glsl"
#include "common.glsl"
#include "bsdf/diffuse.glsl"
#include "integrator/path.glsl"

const uint min_rr = 2;

void main() {
    render(uvec2(gl_LaunchSizeEXT.xy), uvec2(gl_LaunchIDEXT.xy));
}
