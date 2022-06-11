
#version 460
#extension GL_EXT_ray_tracing : require

layout(location = 0) rayPayloadInEXT Payload {
    vec3 rayOrigin;
    vec3 rayDirection;
    vec3 previousNormal;

    vec3 directColor;
    vec3 indirectColor;
    int rayDepth;

    int rayActive;
} payload;

void main() {
    payload.directColor = vec3(0., 1., 0.);
    payload.rayActive = 0;
}
