
#version 460
#extension GL_EXT_ray_tracing : require

layout(location = 0) rayPayloadInEXT Payload {
    vec3 orig;
    vec3 dir;

    vec3 directColor;
    int depth;

    int ray_active;
} payload;

void main() {
    payload.directColor = payload.dir;
}
