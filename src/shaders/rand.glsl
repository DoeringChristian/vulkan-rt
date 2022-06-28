#ifndef RAND_GLSL
#define RAND_GLSL

#define M_PI 3.1415926535897932384626433832795

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

float rand(float seed){
    return fract(sin(seed * 012.9898) * 43758.5453);
}

float rand(vec2 seed) {
    return rand(dot(seed, vec2(0.129898, 0.78233)));
}

float rand(vec3 seed){
    return rand(dot(seed, vec3(0.8556145372, 0.6562710953, 0.4043027253)));
}
/*
float rand(vec4 seed){
    return rand4(seed).x;
}
*/

vec2 rand2(float seed){
    return vec2(rand(seed * 0.8556145372), rand(seed * 0.6562710953));
}
vec2 rand2(vec2 seed){
    return vec2(rand(dot(seed, vec2(0.8556145372, 0.6562710953))), rand(dot(seed, vec2(0.637758729, 0.842307473))));
}
vec2 rand2(vec3 seed){
    return vec2(rand(dot(seed, vec3(0.8556145372, 0.6562710953, 0.4043027253))), rand(dot(seed, vec3(0.637758729, 0.842307473, 0.546435341))));
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


vec3 uniform_sphere(vec3 seed){
    vec2 uv = rand2(seed);
    float theta = acos(1. - 2. * uv.x);
    float phi = 2 * M_PI *    uv.y;
    return vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );
}
vec2 uniform_sphere_uv(vec3 seed){
    vec2 uv = rand2(seed);
    float theta = acos(1. - 2. * uv.x);
    float phi = 2 * M_PI *    uv.y;
    return vec2(theta, phi);
}
vec3 uniform_hemisphere(vec3 seed){
    vec2 uv = rand2(seed);
    float theta = acos(1. - uv.x);
    float phi = 2 * M_PI * uv.y;
    return vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );
}
vec3 uniform_hemisphere_alligned(vec3 normal, vec3 seed){
    vec3 sphere = uniform_sphere(seed);
    if (dot(normal , sphere) <= 0.){
        return reflect(sphere, normal);
    }
    return sphere;
}
vec2 uniform_hemisphere_uv(vec3 seed){
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
