#ifndef PAYLOAD_GLSL
#define PAYLOAD_GLSL

struct Payload{
    vec3 orig;
    vec3 dir;
    
    vec3 color;
    vec3 attenuation;

    float prop;
   
    int depth;
    int ray_active;
};

#endif //PAYLOAD_GLSL
