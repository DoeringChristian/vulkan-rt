#ifndef PUSH_CONSTANT_GLSL
#define PUSH_CONSTANT_GLSL

layout(push_constant) uniform PushConstants{
    uint camera;
    uint max_depth;
    uint rr_depth;
    uint seed;
}push_constant;

#endif //PUSH_CONSTANT_GLSL
