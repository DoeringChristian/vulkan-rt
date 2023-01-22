#ifndef COMMON_GLSL
#define COMMON_GLSL

struct Instance
{
    row_major float4x4 to_world;
    uint mesh;
    uint material;
    int emitter;
};

struct Mesh
{
    uint indices;
    uint indices_count;
    uint positions;
    uint normals;
    uint uvs;
};

struct Texture
{
    float3 val;
    uint _texture;
    uint ty;
};

struct Emitter
{
    Texture emission;
    uint instance;
    uint ty;
};

struct Material
{
    Texture base_color;
    Texture emission;
    Texture normal;
    Texture metallic_roughness;
    Texture transmission;
};

struct Camera
{
    row_major float4x4 to_world;
    row_major float4x4 to_view;
    float near_clip;
    float far_clip;
};

struct PushConstants{
    uint camera;
    uint max_depth;
    uint rr_depth;
    uint seed;
};

#endif //COMMON_GLSL
