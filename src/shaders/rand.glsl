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
	PCG random number generators ported to an NPM package, so that you can require it from glslify.
	The code is based (mostly copied) from https://www.shadertoy.com/view/XlGcRh by Mark Jarzynski.
	References:
	* Mark Jarzynski and Marc Olano, Hash Functions for GPU Rendering, Journal of
	  Computer Graphics Techniques (JCGT), vol. 9, no. 3, 21-38, 2020
	  Available online http://jcgt.org/published/0009/03/02/
	* https://www.pcg-random.org/
*/

uint pcg(uint v) {
	uint state = v * uint(747796405) + uint(2891336453);
	uint word = ((state >> ((state >> uint(28)) + uint(4))) ^ state) * uint(277803737);
	return (word >> uint(22)) ^ word;
}

float prng (float p) {
	return float(pcg(uint(p))) / float(uint(0xffffffff));
}

float prng (vec2 p) {
	return float(pcg(pcg(uint(p.x)) + uint(p.y))) / float(uint(0xffffffff));
}

uvec3 pcg(uvec3 v) {
	v = v * uint(1664525) + uint(1013904223);

	v.x += v.y * v.z;
	v.y += v.z * v.x;
	v.z += v.x * v.y;

	v ^= v >> uint(16);

	v.x += v.y * v.z;
	v.y += v.z * v.x;
	v.z += v.x * v.y;

	return v;
}

vec3 prng (vec3 p) {
	return vec3(pcg(uvec3(p))) / float(uint(0xffffffff));
}

uvec4 pcg(uvec4 v) {
	v = v * uint(1664525) + uint(1013904223);

	v.x += v.y * v.w;
	v.y += v.z * v.x;
	v.z += v.x * v.y;
	v.w += v.y * v.z;

	v.x += v.y * v.w;
	v.y += v.z * v.x;
	v.z += v.x * v.y;
	v.w += v.y * v.z;

	v = v ^ (v >> uint(16));

	return v;
}

vec4 prng (vec4 p) {
	return vec4(pcg(uvec4(p))) / float(uint(0xffffffff));
}

float randf(inout uint seed){
    seed = pcg(seed);
    return uint_to_unit_float(seed);
}
uint randu(inout uint seed){
    seed = pcg(seed);
    return seed;
}
/*
float rand(vec4 seed){
    return rand4(seed).x;
}
*/

vec2 rand2f(inout uint seed){
    seed = pcg(seed);
    float u = uint_to_unit_float(seed);
    seed = pcg(seed);
    float v = uint_to_unit_float(seed);
    return vec2(u, v);
}
/*
vec2 rand2(vec4 seed){
    return rand4(seed).xy;
}
*/

/*
vec3 rand3(float seed){
    return rand4(vec4(seed, 0., 0., 0.)).xyz;
}
vec3 rand3(vec2 seed){
    return rand4(vec4(seed, 0., 0.)).xyz;
}
vec3 rand3(vec3 seed){
    return rand4(vec4(seed, 0.)).xyz;
}
vec3 rand3(vec4 seed){
    return rand4(seed).xyz;
}

vec4 rand4(float seed){
    return rand4(vec4(seed, 0., 0., 0.));
}
vec4 rand4(vec2 seed){
    return rand4(vec4(seed, 0., 0.));
}
vec4 rand4(vec3 seed){
    return rand4(vec4(seed, 0.));
}
vec4 rand4(vec4 seed){
    return prng(seed);
}
*/


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
