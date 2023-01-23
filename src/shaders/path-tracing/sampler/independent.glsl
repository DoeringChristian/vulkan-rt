#ifndef SAMPLER_GLSL
#include "rand.glsl"

#define SAMPLER_GLSL
float next_1d(){
    return next_float();
}
vec2 next_2d(){
    return vec2(next_1d(), next_1d());
}

#endif //SAMPLER_GLSL
