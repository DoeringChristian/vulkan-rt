#ifndef BINDINGS_GLSL
#define BINDINGS_GLSL

#include "common.glsl"

layout(location = 0) rayPayloadEXT Payload payload;
layout(location = 1) rayPayloadEXT bool isShadow;
layout(set = 0, binding = 0) uniform accelerationStructureEXT accel;

layout(std140, set = 0, binding = 1) buffer Indices{
    uint indices[];
};
layout(std140, set = 0, binding = 2) buffer Positions{
    vec3 positions[];
};
layout(std140, set = 0, binding = 3) buffer Normals{
    vec3 normals[];
};
layout(std140, set = 0, binding = 4) buffer UVs{
    vec2 uvs[];
};
    
layout(std140, set = 0, binding = 5) buffer Instances{
    Instance instances[];
};
layout(std140, set = 0, binding = 6) buffer Meshes{
    Mesh meshes[];
};
layout(std140, set = 0, binding = 7) buffer Emitters{
    Emitter emitters[];
};
layout(std140, set = 0, binding = 8) buffer Materials{
    Material materials[];
};
layout(std140, set = 0, binding = 9) buffer Cameras{
    Camera cameras[];
};
layout(set = 0, binding = 10) uniform sampler2D textures[];

layout(set = 1, binding = 0, rgba32f) uniform image2D image;

layout(push_constant) uniform PushConstants{
    uint camera;
    uint max_depth;
    uint rr_depth;
    uint seed;
}push_constant;


#endif //BINDINGS_GLSL
