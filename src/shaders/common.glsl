#ifndef COMMON_GLSL
#define COMMON_GLSL

struct Material {
    vec4 albedo;
    vec4 mr;
    vec4 emission;
    uint albedo_tex;
    uint albedo_texco;
    uint mr_tex;
    uint mr_texco;
    uint emission_tex;
    uint emission_texco;
    uint normal_tex;
    uint normal_texco;
};

struct InterMaterial{
    vec4 albedo;
    vec2 mr;
    vec4 emission;
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
    uint positions;
    uint normals;
    uint tex_coords;
    uint tex_coords_num;
};

struct Payload{
    vec3 orig;
    vec3 dir;
    
    vec3 color;
    vec3 attenuation;

    float prop;
   
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
};

struct Vertex{
    vec4 pos;
    vec4 normal;
    vec4 uv;
};

#endif //COMMON_GLSL
