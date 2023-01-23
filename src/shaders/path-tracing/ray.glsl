#ifndef RAY_GLSL
#define RAY_GLSL
    
struct Ray{
    vec3 o;
    vec3 d;
    float tmin;
    float tmax;
};

#endif //RAY_GLSL
