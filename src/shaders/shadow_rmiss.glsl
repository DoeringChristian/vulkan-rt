
#version 460
#extension GL_EXT_ray_tracing : require

layout(location = 1) rayPayloadInEXT bool shadow_payload;

void main() {
    shadow_paylod = false;
}
