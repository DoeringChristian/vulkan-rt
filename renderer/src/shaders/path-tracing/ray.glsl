#ifndef RAY_GLSL
#define RAY_GLSL
    
struct Ray{
    vec3 o;
    vec3 d;
    float tmin;
    float tmax;
};

Ray ray_from_to(vec3 from, vec3 to){
    float dist = length(to - from);
    return Ray(from, (to - from)/dist, 0.001, dist - 0.001);
}

#endif //RAY_GLSL
