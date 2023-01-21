#ifndef PERSPECTIVE_GLSL
#define PERSPECTIVE_GLSL

#include "bindings.glsl"

Ray sample_ray(vec2 sample_pos){
    
    Camera camera = cameras[push_constant.camera];

    mat4 view_to_camera = inverse(camera.to_view);

    vec3 near_p = (view_to_camera * vec4(sample_pos.xy, 0., 1.)).xyz;

    
    vec3 o = camera.to_world[3].xyz;
    vec3 d = normalize(near_p);
    
    Ray ray;
    
    ray.o = o;
    ray.d = -normalize((camera.to_world * vec4(d, 0.))).xyz;
    
    float near_t = camera.near_clip / -d.z;
    float far_t = camera.far_clip / -d.z;

    ray.tmin = 0.001;
    ray.tmax = 10000.;
    return ray;
}

#endif //PERSPECTIVE_GLSL