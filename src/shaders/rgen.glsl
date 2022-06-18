#version 460
#extension GL_EXT_ray_tracing : require

#define M_PI 3.1415926535897932384626433832795

layout(location = 0) rayPayloadEXT Payload {
    vec3 orig;
    vec3 dir;
    //vec3 prev_norm;

    vec3 color;
    int depth;

    int ray_active;
} payload;

layout(set = 0, binding = 0) uniform accelerationStructureEXT topLevelAS;

layout(set = 0, binding = 1, rgba32f) uniform image2D image;

float random(vec2 uv, float seed) {
    return fract(sin(mod(dot(uv, vec2(12.9898, 78.233)) + 1113.1 * seed, M_PI)) *
                 43758.5453);
}

void main() {
    vec2 uv = gl_LaunchIDEXT.xy
        + vec2(random(gl_LaunchIDEXT.xy, 0), random(gl_LaunchIDEXT.xy, 1));
    uv /= vec2(gl_LaunchSizeEXT.xy);
    uv = (uv * 2.0f - 1.0f) * vec2(1.0f, -1.0f);

    payload.orig = vec3(3., 0., 0.);
    payload.dir = normalize(vec3(-1, uv.x, uv.y));
    //payload.prev_norm = vec3(0.0, 0.0, 0.0);

    payload.color = vec3(0.0, 0.0, 0.0);
    payload.depth = 0;

    payload.ray_active = 1;

    for (int x = 0; x < 1; x++) {
        traceRayEXT(topLevelAS, gl_RayFlagsOpaqueEXT, 0xFF, 0, 0, 0,
                    payload.orig, 0.001, payload.dir, 10000.0, 0);
    }

    vec4 color = vec4(payload.color, 1.0);

    imageStore(image, ivec2(gl_LaunchIDEXT.xy), color);
}
