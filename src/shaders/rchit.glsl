
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable

#define M_PI 3.1415926535897932384626433832795

struct Material {
    vec4 diffuse;
    vec4 mra;
};

hitAttributeEXT vec2 hitCoordinate;

layout(location = 0) rayPayloadInEXT Payload {
    vec3 orig;
    vec3 dir;
    //vec3 prev_norm;

    vec3 directColor;
    int depth;

    int ray_active;
} payload;

layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
layout(binding = 2, set = 0) buffer Materials{
    Material materials[];
};

float random(vec2 uv, float seed) {
    return fract(sin(mod(dot(uv, vec2(12.9898, 78.233)) + 1113.1 * seed, M_PI)) *
                 43758.5453);
}

vec3 uniformSampleHemisphere(vec2 uv) {
    float z = uv.x;
    float r = sqrt(max(0, 1.0 - z * z));
    float phi = 2.0 * M_PI * uv.y;

    return vec3(r * cos(phi), z, r * sin(phi));
}

vec3 alignHemisphereWithCoordinateSystem(vec3 hemisphere, vec3 up) {
    vec3 right = normalize(cross(up, vec3(0.0072f, 1.0f, 0.0034f)));
    vec3 forward = cross(right, up);

    return hemisphere.x * right + hemisphere.y * up + hemisphere.z * forward;
}

void main() {
    if (payload.ray_active == 0) {
        return;
    }

    Material mat = materials[gl_InstanceCustomIndexEXT];

    payload.directColor = mat.diffuse.xyz;

    payload.orig = vec3(0., 0., 0.);
    payload.dir = vec3(0., 1., 0.);
    //payload.prev_norm = vec3(0., 0., 1.);

    payload.depth += 1;
}
