#ifndef RESTIR_PUSHCONSTANT_GLSL
#define RESTIR_PUSHCONSTANT_GLSL

layout(push_constant) uniform PushConstants{
    uint camera;
    uint max_depth;
    uint rr_depth;
    uint seed;
    uint do_spatiotemporal;
}push_constant;

#endif //RESTIR_PUSHCONSTANT_GLSL
