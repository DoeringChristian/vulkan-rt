#ifndef BINDINGS_GLSL
#define BINDINGS_GLSL

layout(set = 0, binding = 0) uniform accelerationStructureEXT accel;

// NOTE: std140 forces 16 byte array stride for uints.
// AsStd140 does not reflect this and therefore I removed the std140 qualifier.
// Additionally, why is the std140 alignment 16 byte (seems a bit excessive).
layout(set = 0, binding = 1) buffer Indices{
    uint indices[];
};
layout(std140, set = 0, binding = 2) buffer Positions{
    vec3 positions[];
};
layout(std140, set = 0, binding = 3) buffer Normals{
    vec3 normals[];
};
layout(set = 0, binding = 4) buffer UVs{
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

layout(push_constant) uniform PushConstants{
    uint camera;
    uint max_depth;
    uint rr_depth;
    uint seed;
}push_constant;


#endif //BINDINGS_GLSL
