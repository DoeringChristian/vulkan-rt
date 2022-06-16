
#version 460
#extension GL_EXT_ray_tracing : require

layout(location = 0) rayPayloadInEXT Payload {
    vec3 orig;
    vec3 dir;
    //vec3 prev_norm;

    vec3 directColor;
    //vec3 indirectColor;
    int depth;

    int ray_active;
} payload;

void main() {
    payload.directColor = vec3(0., 1., 0.);
}
