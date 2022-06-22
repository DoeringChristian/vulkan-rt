#ifndef COMMON_GLSL
#define COMMON_GLSL

struct Material {
    vec4 albedo;
    vec4 mra;
    vec4 emission;
};

struct Instance{
    uint mat_index;
    uint indices;
    uint positions;
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
