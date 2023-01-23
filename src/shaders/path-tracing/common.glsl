#ifndef COMMON_GLSL
#define COMMON_GLSL

// Structs used between shader and rust

struct Texture{
    vec3 val;
    uint texture;
    uint ty;
};
#define TEXTURE_TY_CONSTANT 0
#define TEXTURE_TY_IMAGE 1

struct Mesh{
    uint indices;
    uint indices_count;
    uint positions;
    uint normals;
    uint uvs;
};
struct Instance{
    mat4 to_world;
    uint mesh;
    uint material;
    int emitter;
};
struct Emitter{
    Texture emission;
    uint instance;
    uint ty;
};
#define EMITTER_TY_NONE 0
#define EMITTER_TY_ENV 1
#define EMITTER_TY_AREA 2
struct Material{
    Texture base_color;
    Texture emission;
    Texture normal;
    Texture metallic_roughness;
    Texture transmission;
};
struct Camera{
    mat4 to_world;
    mat4 to_view;
    float near_clip;
    float far_clip;
};

struct RestirSample{
    vec3 pv;
    vec3 nv;
    vec3 ps;
    vec3 ns;

    vec3 Lo_hat;
    vec3 random;
};

struct RestirReservoir{
    RestirSample s;
    float w;
    float W;
    float M;
};

// Shared between shaders

struct Payload{
    uint valid;
    uint instance;
    uint primitive;
    vec3 barycentric;
};


// Internal structs

struct BSDFSample{
    vec3 wo;
    float pdf;
};


#endif //COMMON_GLSL
