#ifndef RAND_GLSL
#define RAND_GLSL

#define M_PI 3.1415926535897932384626433832795

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

float randf(inout uint seed){
    seed = pcg(seed);
    return uint_to_unit_float(seed);
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
