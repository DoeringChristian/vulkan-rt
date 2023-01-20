#ifndef WARP_GLSL
#define WARP_GLSL

#include "math.glsl"

vec2 square_to_uniform_disk_concentric(vec2 s){
    float x = 2. * s.x - 1;
    float y = 2. * s.y - 1;

    float r;
    float phi;
    if (x == 0 && y == 0){
        r = 0;
        phi = 0;
    }else if(x*x > y*y){
        r = x;
        phi = (PI / 4.) * (y / x);
    }else{
        r = y;
        phi = (PI/2.) - (x / y) * (PI / 4.);
    }

    return vec2(r * cos(phi), r * sin(phi));
}

vec3 square_to_cosine_hemisphere(vec2 s){
    vec2 p = square_to_uniform_disk_concentric(s);

    float z = sqrt(1. - dot(p, p));

    return vec3(p.x, p.y, z);
}

float square_to_cosine_hemisphere_pdf(vec3 v){
    return 1./PI * v.z;
}

#endif //WARP_GLSL
