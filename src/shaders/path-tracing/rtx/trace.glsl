#ifndef TRACE_GLSL
#define TRACE_GLSL

#include "ray.glsl"
#include "interaction.glsl"

SurfaceInteraction ray_intersect(in Ray ray){
    SurfaceInteraction si;
    payload.valid = 0;
    traceRayEXT(accel, gl_RayFlagsOpaqueEXT, 0xFF, 0, 0, 0,
                ray.o, ray.tmin, ray.d, ray.tmax, 0);


    // DEBUG:
    // L = ray.d;
    // break;

    if (payload.valid == 0){
        si.valid = false;
        return si;
    }
    si.valid = true;
    si.instance = payload.instance;
    si.primitive = payload.primitive;
    si.barycentric = payload.barycentric;


    finalize_surface_interaction(si, ray);
    return si;
}

bool ray_test(in Ray ray){
    shadow_payload = true;
    uint shadowRayFlags = gl_RayFlagsTerminateOnFirstHitEXT
        | gl_RayFlagsOpaqueEXT
        | gl_RayFlagsSkipClosestHitShaderEXT;
    traceRayEXT(
            accel,
            shadowRayFlags,
            0xFF, 
            0, 
            0, 
            1, 
            ray.o, 
            ray.tmin,
            ray.d, 
            ray.tmax,
            1
        );
    return shadow_payload;
}

#endif //TRACE_GLSL
