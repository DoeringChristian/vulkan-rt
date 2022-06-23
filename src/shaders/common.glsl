#ifndef COMMON_GLSL
#define COMMON_GLSL

struct Material {
    vec4 albedo;
    vec4 mra;
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

#endif //COMMON_GLSL
