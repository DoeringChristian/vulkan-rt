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

    float z = sqrt(max(0., 1. - dot(p, p)));

    return vec3(p.x, p.y, z);
}

float square_to_cosine_hemisphere_pdf(vec3 v){
    return v.z > 0.? (1./PI * v.z):0.;
}

// =======================================================================

vec2 square_to_uniform_triangle(vec2 s){
    float t = sqrt(max(0., 1. - s.x));
    return vec2(1. - t, t * s.y);
}

vec2 uniform_traingle_to_square(vec2 p){
    float t = 1. - p.x;
    return vec2(1. - t * t, p.y / t);
}

float square_to_uniform_triangle_pdf(vec2 p){
    return p.x < 0. || p.y < 0. || (p.x + p.y > 1.)?0.:2.;
}

uint sample_reuse(inout float value, uint num){
    float scaled_index = value * float(num);
    uint index = uint(scaled_index);
    value = scaled_index - floor(scaled_index);
    return index;
}

#endif //WARP_GLSL
