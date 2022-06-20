
#version 460
#extension GL_EXT_ray_tracing : require

#include "payload.glsl"

layout(location = 0) rayPayloadInEXT Payload payload;

void main() {
    payload.color *= 0;
    //payload.color *= payload.dir * 100.;
}
