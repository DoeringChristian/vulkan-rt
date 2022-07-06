#ifndef COMMON_GLSL
#define COMMON_GLSL

#define M_PI 3.1415926535897932384626433832795
#define INV_PI (1. / M_PI)
#define TWO_PI (2. * M_PI)

struct Material {
    vec4 albedo;
    vec4 mr;
    vec4 emission;
    float transmission;
    float transmission_roughness;
    float ior;
    uint _pack;
    uint albedo_tex;
    uint mr_tex;
    uint emission_tex;
    uint normal_tex;
};

struct HitInfo{
    vec4 albedo;
    vec4 emission;
    float metallic;
    float roughness;
    float transmission;
    float ior;

    
    vec3 pos;
    vec3 wo;
    // Geometry Normal
    vec3 g;
    // Texture Normal
    vec3 n;
    float dist;
};

#define INDEX_UNDEF 0xffffffff
struct Instance{
    //mat4 trans;
    vec4 trans0;
    vec4 trans1;
    vec4 trans2;
    vec4 trans3;
    uint mat_index;
    uint indices;
    uint vertices;

    uint normal_uv_mask;
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

struct Evaluation{
    vec3 brdf;
    vec3 dir;
};

/*
Vertex interpolate_vertex(Vertex vert0, Vertex vert1, Vertex vert2, vec3 barycentric){
    
}
*/

#endif //COMMON_GLSL
