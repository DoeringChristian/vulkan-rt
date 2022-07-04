#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "rand.glsl"
#include "common.glsl"

layout(location = 0) rayPayloadEXT Payload payload;
layout(set = 0, binding = 0) uniform accelerationStructureEXT topLevelAS;

layout(set = 1, binding = 0, rgba32f) uniform image2D image;

layout(push_constant) uniform PushConstants{
    Camera camera;
};

void main() {
    uint N = camera.fc;
    vec2 uv = gl_LaunchIDEXT.xy;

    uint seed = N;
    seed = randu(seed) + gl_LaunchIDEXT.x;
    seed = randu(seed) + gl_LaunchIDEXT.y;
    seed = randu(seed);
    
    //vec2 roff = rand2(vec3(float(N), uv.x, uv.y));
    vec2 roff = rand2f(seed);
    uv += roff;
    uv /= vec2(gl_LaunchSizeEXT.xy);
    uv = (uv * 2. - 1.) * vec2(1., 1.);
    uv *= tan(camera.fov/2.);
    vec3 up = camera.up.xyz;
    vec3 right = camera.right.xyz;
    vec3 forward = normalize(cross(up, right));

    //payload.orig = vec3(1., 0., 0.);
    payload.orig = camera.pos.xyz;
    payload.dir = normalize(up * uv.x + right * uv.y + forward);

    payload.color = vec3(0.);
    payload.attenuation = vec3(1.);
    payload.ior = 1.;

    //payload.prop = 1.;
    
    payload.seed = seed;
    payload.depth = 0;
    payload.ray_active = 1;

    vec3 color = imageLoad(image, ivec2(gl_LaunchIDEXT.xy)).xyz;
    
    /*
    vec2 uv = gl_LaunchIDEXT.xy;
    vec2 roff = rand2(vec3(float(N), uv.x, uv.y));
    uv += roff;
    uv /= vec2(gl_LaunchSizeEXT.xy);
    uv = (uv * 2. - 1.) * vec2(1., -1.);
    uv *= 0.7;
    */
    for (int x = 0; x < camera.depth; x++) {
    //for (int x = 0; x < 1; x++) {
        traceRayEXT(topLevelAS, gl_RayFlagsOpaqueEXT, 0xFF, 0, 0, 0,
                    payload.orig, 0.001, payload.dir, 10000.0, 0);
    }
    // DEBUG: boost light:
    //payload.color *= 10.;
    //payload.color = payload.color/(payload.color + vec3(1.));
    //payload.color = pow(payload.color, vec3(1.0/2.2));
    if (N == 0){
        color = payload.color;
    }
    else{
        color = 1/float((N + 1)) * payload.color + float(N)/float(N + 1)*color; 
    }

    //vec4 color = vec4(payload.color, 1.0);

    imageStore(image, ivec2(gl_LaunchIDEXT.xy), vec4(color, 0.));
}
