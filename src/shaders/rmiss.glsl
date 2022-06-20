
#version 460
#extension GL_EXT_ray_tracing : require

layout(location = 0) rayPayloadInEXT Payload {
    vec3 orig;
    vec3 dir;

    vec3 color;
    int depth;

    int ray_active;
} payload;

void main() {
    //payload.color *= payload.dir * 100.;
}
