#version 460
#extension GL_EXT_ray_tracing : require

#define M_PI 3.1415926535897932384626433832795

layout(location = 0) rayPayloadEXT Payload {
    vec3 rayOrigin;
    vec3 rayDirection;
    vec3 previousNormal;

    vec3 directColor;
    vec3 indirectColor;
    int rayDepth;

    int rayActive;
} payload;

layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;

layout(binding = 1, set = 0, rgba32f) uniform image2D image;

float random(vec2 uv, float seed) {
    return fract(sin(mod(dot(uv, vec2(12.9898, 78.233)) + 1113.1 * seed, M_PI)) *
                 43758.5453);
}

void main() {
    vec2 uv = gl_LaunchIDEXT.xy
        + vec2(random(gl_LaunchIDEXT.xy, 0), random(gl_LaunchIDEXT.xy, 1));
    uv /= vec2(gl_LaunchSizeEXT.xy);
    uv = (uv * 2.0f - 1.0f) * vec2(1.0f, -1.0f);

    payload.rayOrigin = vec3(3., 0., 0.);
    payload.rayDirection = normalize(vec3(-1, uv.x, uv.y));
    payload.previousNormal = vec3(0.0, 0.0, 0.0);

    payload.directColor = vec3(0.0, 0.0, 1.0);
    payload.indirectColor = vec3(0.0, 0.0, 0.0);
    payload.rayDepth = 0;

    payload.rayActive = 1;

    for (int x = 0; x < 1; x++) {
        traceRayEXT(topLevelAS, gl_RayFlagsOpaqueEXT, 0xFF, 0, 0, 0,
                    payload.rayOrigin, 0.001, payload.rayDirection, 10000.0, 0);
    }

    vec4 color = vec4(payload.directColor, 1.0);

    imageStore(image, ivec2(gl_LaunchIDEXT.xy), color);
}
