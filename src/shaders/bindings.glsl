#ifndef BINDINGS_GLSL
#define BINDINGS_GLSL

#include "common.glsl"

layout(location = 0) rayPayloadEXT Payload payload;
layout(location = 1) rayPayloadEXT bool isShadow;
layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
layout(std140, set = 0, binding = 1) buffer Instances{
    InstanceData instances[];
};
layout(std140, set = 0, binding = 2) buffer Materials{
    MaterialData materials[];
};
layout(set = 0, binding = 3) uniform sampler2D textures[];
layout(set = 0, binding = 4) buffer Lights{
    uvec4 count;
    LightData l[];
}lights;

layout(set = 1, binding = 0, rgba32f) uniform image2D image;

layout(push_constant) uniform PushConstants{
    Camera camera;
};

// Buffer References
layout(buffer_reference, scalar) buffer Indices{
    uint i[];
};
layout(buffer_reference, scalar) buffer Vertices{
    Vertex v[];
};


#endif //BINDINGS_GLSL
