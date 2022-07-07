#ifndef RAND_GLSL
#define RAND_GLSL

#include "math.glsl"

float uint_to_unit_float(uint u){
    const uint mantissaMask = 0x007FFFFFu;
    const uint one          = 0x3F800000u;
    u &= mantissaMask;
    u |= one;
    float  r2 = uintBitsToFloat(u);
    return r2 - 1.0;
}

/*
Pcg Hashing algorithm copied from https://www.shadertoy.com/view/XlGcRh.
 https://www.pcg-random.org/
*/
uint pcg(uint v)
{
	uint state = v * 747796405u + 2891336453u;
	uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
	return (word >> 22u) ^ word;
}

// Rng state
uvec4 seed;
//RNG from code by Moroz Mykhailo (https://www.shadertoy.com/view/wltcRS)
void pcg4d(inout uvec4 v)
{
    v = v * 1664525u + 1013904223u;
    v.x += v.y * v.w; v.y += v.z * v.x; v.z += v.x * v.y; v.w += v.y * v.z;
    v = v ^ (v >> 16u);
    v.x += v.y * v.w; v.y += v.z * v.x; v.z += v.x * v.y; v.w += v.y * v.z;
}

float randf(inout uint seed){
    seed = pcg(seed);
    return uint_to_unit_float(seed);
}
float randf(){
    pcg4d(seed);
    return float(seed.x)/float(0xffffffffu);
}
uint randu(inout uint seed){
    seed = pcg(seed);
    return seed;
}

vec2 rand2f(inout uint seed){
    seed = pcg(seed);
    float x = uint_to_unit_float(seed);
    seed = pcg(seed);
    float y = uint_to_unit_float(seed);
    return vec2(x, y);
}
vec3 rand3f(inout uint seed){
    seed = pcg(seed);
    float x = uint_to_unit_float(seed);
    seed = pcg(seed);
    float y = uint_to_unit_float(seed);
    seed = pcg(seed);
    float z = uint_to_unit_float(seed);
    return vec3(x, y, z);
}


vec3 uniform_sphere(inout uint seed){
    vec2 uv = rand2f(seed);
    float theta = acos(1. - 2. * uv.x);
    float phi = 2 * M_PI *    uv.y;
    return vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );
}
vec2 uniform_sphere_uv(inout uint seed){
    vec2 uv = rand2f(seed);
    float theta = acos(1. - 2. * uv.x);
    float phi = 2 * M_PI *    uv.y;
    return vec2(theta, phi);
}
vec3 uniform_hemisphere(inout uint seed){
    vec2 uv = rand2f(seed);
    float theta = acos(1. - uv.x);
    float phi = 2 * M_PI * uv.y;
    return vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );
}
vec3 uniform_hemisphere_alligned(vec3 normal, inout uint seed){
    vec3 sphere = uniform_sphere(seed);
    if (dot(normal , sphere) <= 0.){
        return reflect(sphere, normal);
    }
    return sphere;
}
vec3 cosine_hemisphere(inout uint seed){
    float r = sqrt(randf(seed));
    float phi = randf(seed) * 2. * M_PI;

    float x = r * cos(phi);
    float y = r * sin(phi);

    return vec3(x, y, sqrt(1. - x*x - y*y));
}

vec2 uniform_hemisphere_uv(inout uint seed){
    vec2 uv = uniform_sphere_uv(seed);
    if (uv.x > 0){
        return uv;
    }
    else{
        return vec2(-uv.x, uv.y);
    }
}

vec3 allign_hemisphere(vec3 hemisphere, vec3 up){
    vec3 right = normalize(cross(up, vec3(0.0072, 1., 0.0034)));
    vec3 forward = cross(right, up);

    return hemisphere.x * forward + hemisphere.y * right + hemisphere.z * up;
}

#endif //RAND_GLSL
