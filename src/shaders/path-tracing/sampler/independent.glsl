#ifndef SAMPLER_GLSL
#define SAMPLER_GLSL

#include "rand.glsl"

struct SampleGenerator{
    PCG pcg;
};

SampleGenerator sample_generator(uint seed, uint idx){
    return SampleGenerator(pcg(sample_tea_32(push_constant.seed, idx)));
}

float next_1d(inout SampleGenerator self){
    return next_float(self.pcg);
}
vec2 next_2d(inout SampleGenerator self){
    return vec2(next_1d(self), next_1d(self));
}

#endif //SAMPLER_GLSL
