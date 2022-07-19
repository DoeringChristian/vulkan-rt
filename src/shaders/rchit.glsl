
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "rand.glsl"
#include "common.glsl"
#include "disney_bsdf01.glsl"


hitAttributeEXT vec2 hit_co;

layout(location = 0) rayPayloadInEXT Payload payload;
layout(location = 1) rayPayloadEXT bool isShadow;

layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
//layout(set = 0, binding = 1, rgba32f) uniform image2D image;
/*
layout(std140, set = 0, binding = 1) buffer Instances{
    InstanceData instances[];
};
layout(std140, set = 0, binding = 2) buffer Materials{
    MaterialData materials[];
};
layout(set = 0, binding = 3) uniform sampler2D textures[];
layout(set = 0, binding = 4) buffer Lights{
    uvec4 count;
    LightData l[];
}lights;

layout(buffer_reference, scalar) buffer Indices{
    uint i[];
};
layout(buffer_reference, scalar) buffer Vertices{
    Vertex v[];
};

mat3 compute_TBN(vec2 duv0, vec2 duv1, vec3 dpos0, vec3 dpos1, vec3 n){
    float r = 1./(duv0.x * duv1.y - duv0.y * duv1.x);
    vec3 t = (dpos0 * duv1.y - dpos1 * duv0.y)*r;
    vec3 b = (dpos1 * duv0.x - dpos0 * duv1.x)*r;
    return mat3(t, b, n);
}
*/


//const uint min_rr = 2;

void main() {
    payload.hit_co = hit_co;
    payload.instanceIndex = gl_InstanceCustomIndexEXT;
    payload.primitiveID = gl_PrimitiveID;
    /*
    if (payload.ray_active == 0) {
        return;
    }
    init_seed(payload.seed);

    HitInfo hit;
    Material mat;
    Medium med;
    
    hitInfo(hit, mat, med);


    //===========================================================
    // Call BRDF functions:
    //===========================================================

    
    // Sample bsdf and scattering function
    vec3 radiance;
    float pf;
    vec3 f;
    bool mediumEntered;
    sample_shader(
            hit, 
            mat, 
            payload.med, 
            payload.orig, 
            payload.dir, 
            mediumEntered,
            radiance, 
            f, 
            pf);
    if(mediumEntered){
        payload.med = mat.med;
    }
        
    // Sample light
    vec3 g;
    float pg = 0.;

    uint lightIndex = randu(lights.count.x);
    SampledLight light = sampleLight(lights.l[lightIndex]);

    isShadow = true;
    uint shadowRayFlags = gl_RayFlagsTerminateOnFirstHitEXT
        | gl_RayFlagsOpaqueEXT
        | gl_RayFlagsSkipClosestHitShaderEXT;
    traceRayEXT(
            tlas,
            shadowRayFlags,
            0xFF, 
            0, 
            0, 
            1, 
            hit.pos, 
            0.001,
            normalize(light.pos.xyz - hit.pos), 
            length(light.pos.xyz - hit.pos) - 0.001,
            1
        );

    if (!isShadow){
        eval_shader(
                hit,
                mat,
                light.pos.xyz,
                g,
                pg
            );

        pg *= float(lights.count.x);
    }
    // DEBUG:
    //isShadow = true;

    // Combine samples (light and  bsdf) using MIS
    payload.radiance += radiance * payload.throughput;
    if(pg > 0. && !isShadow){
        float misWeight = PowerHeuristic(pg, pf); // Calculate misWeight for light source sampling
        payload.radiance += light.emission.rgb * payload.throughput * misWeight * g / pg;
    }
    if(pf > 0.){
        float misWeight; // Calculate misWeight for bsdf sampling
        if (isShadow){
            misWeight = 1.;
        } else{
            misWeight = PowerHeuristic(pf, pg);
        }
        payload.throughput *= misWeight * f / pf;
    }
    
    //===========================================================
    // Throughput Russian Roulette:
    //===========================================================
    //p_{RR} = max_{RGB}\leftb( \prod_{d = 1}^{D-1} \left({f_r(x_d, w_d \rightarrow v_d) cos(\theta_d)) \over p(w_d)p_{RR_d}}\right)\right)
    float p_rr = max(payload.throughput.r, max(payload.throughput.g, payload.throughput.b));
    if (payload.depth < min_rr){
        p_rr = 1.;
    }
    
    payload.throughput *= 1. / p_rr;
    
    if (randf(payload.seed) >= p_rr){
        payload.ray_active = 0;
        return;
    }
    
    payload.depth += 1;
*/
}
