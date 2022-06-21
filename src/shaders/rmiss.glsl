
#version 460
#extension GL_EXT_ray_tracing : require

#include "common.glsl"

layout(location = 0) rayPayloadInEXT Payload payload;

void main() {
    payload.color += vec3(0.) * payload.attenuation;
    payload.ray_active = 0;
    //payload.color *= payload.dir * 100.;
}
