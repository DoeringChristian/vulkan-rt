#ifndef COMMON_GLSL
#define COMMON_GLSL

#include "math.glsl"

struct MaterialData {
    vec4 albedo;
    vec4 emission;
    float metallic;
    float roughness;
    float transmission;
    float transmission_roughness;
    float ior;
    uint albedo_tex;
    uint mr_tex;
    uint emission_tex;
    uint normal_tex;
};

struct MatInfo{
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

struct HitInfo{
    vec3 pos;
    vec3 wo;
    // Geometry Normal
    vec3 g;
    // Texture Normal
    vec3 n;
    float dist;
};

#define INDEX_UNDEF 0xffffffff
struct InstanceData{
    //mat4 trans;
    vec4 trans0;
    vec4 trans1;
    vec4 trans2;
    vec4 trans3;
    uint mat_index;
    uint mesh_index;
};

#define RAY_TMIN 0.001
struct Payload{
    vec3 orig;
    vec3 dir;
    
    vec3 color;
    vec3 attenuation;
    float ior;

    uint seed;
    int depth;
    int ray_active;
};

struct Camera{
    vec4 up;
    vec4 right;
    vec4 pos;
    float focus;
    float diameter;
    float fov;
    uint fc;
    uint depth;
};

struct Vertex{
    vec4 pos;
    vec4 normal;
    vec4 uv;
};

#define LIGHT_POINT 0
struct LightData{
    vec4 emission;
    vec4 position;
    uint light_type;
};

#endif //COMMON_GLSL
