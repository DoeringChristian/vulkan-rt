#ifndef COMMON_GLSL
#define COMMON_GLSL

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

struct InterMaterial{
    vec4 albedo;
    vec2 mr;
    vec4 emission;
    float transmission;
    float ior;
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

struct Payload{
    vec3 orig;
    vec3 dir;
    
    vec3 color;
    vec3 attenuation;
    float ior;
   
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
