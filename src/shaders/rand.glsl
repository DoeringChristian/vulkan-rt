#ifndef RAND_GLSL
#define RAND_GLSL

#define M_PI 3.1415926535897932384626433832795

float rand(float seed){
    return fract(sin(seed * 012.9898) * 43758.5453);
}

float rand(vec2 seed) {
    return rand(dot(seed, vec2(0.129898, 0.78233)));
}

float rand(vec3 seed){
    return rand(dot(seed, vec3(0.8556145372, 0.6562710953, 0.4043027253)));
}

vec2 rand2(float seed){
    return vec2(rand(seed * 0.8556145372), rand(seed * 0.6562710953));
}
vec2 rand2(vec2 seed){
    return vec2(rand(dot(seed, vec2(0.8556145372, 0.6562710953))), rand(dot(seed, vec2(0.637758729, 0.842307473))));
}
vec2 rand2(vec3 seed){
    return vec2(rand(dot(seed, vec3(0.8556145372, 0.6562710953, 0.4043027253))), rand(dot(seed, vec3(0.637758729, 0.842307473, 0.546435341))));
}


vec3 rand_sphere(vec3 seed){
    vec2 uv = rand2(seed);
    float theta = acos(1. - 2. * uv.x);
    float phi = 2 * M_PI *    uv.y;
    return vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );
}
vec2 rand_sphere_uv(vec3 seed){
    vec2 uv = rand2(seed);
    float theta = acos(1. - 2. * uv.x);
    float phi = 2 * M_PI *    uv.y;
    return vec2(theta, phi);
}
vec3 rand_hemisphere(vec3 normal, vec3 seed){
    vec3 sphere = rand_sphere(seed);
    if (dot(normal , sphere) <= 0.){
        return reflect(sphere, normal);
    }
    return sphere;
}
vec2 rand_hemisphere_uv(vec3 seed){
    vec2 uv = rand_sphere_uv(seed);
    if (uv.x > 0){
        return uv;
    }
    else{
        return vec2(-uv.x, uv.y);
    }
}

#endif //RAND_GLSL
