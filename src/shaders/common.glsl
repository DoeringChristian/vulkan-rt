#ifndef COMMON_GLSL
#define COMMON_GLSL

#include "math.glsl"

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
    uint emitter;
};
struct Emitter{
    Texture iradiance;
    uint instance;
    uint ty;
};
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


struct MaterialInfo{
    vec3 albedo;
    vec3 emission;
    
    float transmission;
    float roughness;
    float metallic;
    float anisotropic;
    float subsurface;
    float specularTint;
    float sheen;
    float sheenTint;
    float clearcoat;
    float clearcoatRoughness;
    float ior;
    float ax;
    float ay;
};

struct SurfaceInteraction{
    vec3 barycentric;
    uint instance;
    uint primitive;
    uint valid;

    vec3 p;
    vec3 n;
    
    vec2 uv;

    mat3 tbn;

    vec3 wi;

    Material material;
};

struct Payload{
    uint instance;
    uint primitive;
    uint valid;
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

#endif //COMMON_GLSL
