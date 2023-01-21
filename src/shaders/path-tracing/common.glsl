#ifndef COMMON_GLSL
#define COMMON_GLSL

#include "util/math.glsl"

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


// Internal structs
struct SurfaceInteraction{
    vec3 barycentric;
    uint instance;
    uint primitive;
    bool valid;

    vec3 p;
    vec3 n;
    
    vec2 uv;

    mat3 tbn;

    vec3 wi;

    //Mesh mesh;
    Material material;
};

struct Payload{
    uint valid;
    uint instance;
    uint primitive;
    vec3 barycentric;
};
    
struct Ray{
    vec3 o;
    vec3 d;
    float tmin;
    float tmax;
};

struct BSDFSample{
    vec3 wo;
    float pdf;
};

struct PositionSample{
    vec3 p;
    vec2 uv;
    vec3 n;
    float pdf;

    vec3 barycentric;

    mat3 tbn;
};

struct DirectionSample{
    vec3 p;
    vec2 uv;
    vec3 n;
    float pdf;
    
    vec3 barycentric;

    mat3 tbn;
    //
    
    vec3 d;
    float dist;
};

#endif //COMMON_GLSL
