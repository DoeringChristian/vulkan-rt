#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "rand.glsl"
#include "bindings.glsl"
#include "common.glsl"
#include "utils.glsl"
#include "disney_bsdf01.glsl"

const uint min_rr = 2;

void main() {
    uint N = camera.fc;
    vec2 uv = gl_LaunchIDEXT.xy;

    init_seed(N);
    init_seed(randu() + gl_LaunchIDEXT.x);
    init_seed(randu() + gl_LaunchIDEXT.y);
    randu();
    
    //vec2 roff = rand2(vec3(float(N), uv.x, uv.y));
    vec2 roff = rand2f();
    uv += roff;
    uv /= vec2(gl_LaunchSizeEXT.xy);
    uv = (uv * 2. - 1.) * vec2(1., 1.);
    uv *= tan(camera.fov/2.);
    vec3 up = camera.up.xyz;
    vec3 right = camera.right.xyz;
    vec3 forward = normalize(cross(up, right));

    //payload.orig = vec3(1., 0., 0.);
    Ray ray;
    ray.orig = camera.pos.xyz;
    ray.dir = normalize(up * uv.x + right * uv.y + forward);

    ray.radiance = vec3(0.);
    ray.throughput = vec3(1.);

    ray.med.color = vec3(1.);
    ray.med.anisotropic = 0.;
    ray.med.density = 0.0;
    
    ray.ior = 1.;
    
    //payload.seed = randu();
    ray.depth = 0;
    
    payload.terminated = 0;

    vec3 color = imageLoad(image, ivec2(gl_LaunchIDEXT.xy)).xyz;
    
    for (ray.depth = 0; ray.depth < camera.depth && payload.terminated != 1; ray.depth++) {
        traceRayEXT(tlas, gl_RayFlagsOpaqueEXT, 0xFF, 0, 0, 0,
                    ray.orig, RAY_TMIN, ray.dir, 10000.0, 0);
        if(payload.terminated == 1){
            break;
        }
        HitInfo hit;
        Material mat;
        Medium med;
        hitInfo(payload.instanceIndex, payload.primitiveID, payload.hit_co, hit, mat);

        vec3 V = -ray.dir;
        
        // Select Medium through which the ray has traveled
        if (dot(hit.g, V) < 0.){
            med = mat.med;
        } else{
            med = ray.med;
        }

        //===========================================================
        // Call BRDF functions:
        //===========================================================

        // Sample bsdf and scattering functions
        vec3 radiance;
        float pf;
        vec3 f;
        bool mediumEntered;
        sample_shader(
                hit,
                mat,
                med,
                ray.orig,
                ray.dir,
                mediumEntered,
                radiance,
                f,
                pf);
        if(mediumEntered){
            ray.med = mat.med;
        }

        // Sample Light
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
                    V,
                    light.pos.xyz,
                    g,
                    pg
                );

            pg *= float(lights.count.x);
        }

        // DEBUG:
        //isShadow = true;

        // Combine samples (light and bsdf) using MIS
        ray.radiance += radiance * ray.throughput;
        if(pg > 0. && !isShadow){
            float misWeight = PowerHeuristic(pg, pf); // Calculate misWeight for light source sampling
            ray.radiance += light.emission.rgb * ray.throughput * misWeight * g / pg;
        }
        if(pf > 0.){
            float misWeight; // Calculate misWeight for bsdf sampling
            if (isShadow){
                misWeight = 1.;
            } else{
                misWeight = PowerHeuristic(pf, pg);
            }
            ray.throughput *= misWeight * f / pf;
        }
        
        //===========================================================
        // Throughput Russian Roulette:
        //===========================================================
        //p_{RR} = max_{RGB}\leftb( \prod_{d = 1}^{D-1} \left({f_r(x_d, w_d \rightarrow v_d) cos(\theta_d)) \over p(w_d)p_{RR_d}}\right)\right)
        float p_rr = max(ray.throughput.r, max(ray.throughput.g, ray.throughput.b));
        if (ray.depth < min_rr){
            p_rr = 1.;
        }

        ray.throughput *= 1. / p_rr;

        if (randf() >= p_rr){
            payload.terminated = 1;
            break;
        }

    }
    if (N == 0){
        color = ray.radiance;
    }
    else{
        color = 1/float((N + 1)) * ray.radiance + float(N)/float(N + 1)*color; 
    }

    //vec4 color = vec4(payload.color, 1.0);

    imageStore(image, ivec2(gl_LaunchIDEXT.xy), vec4(color, 0.));
}
