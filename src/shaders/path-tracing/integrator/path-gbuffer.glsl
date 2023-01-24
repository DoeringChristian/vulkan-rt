#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "common.glsl"
#include "scene-bindings.glsl"

// Ray Tracing Bindings
layout(location = 0) rayPayloadEXT Payload payload;
layout(location = 1) rayPayloadEXT bool shadow_payload;

// Output Images
layout(set = 1, binding = 0, rgba32f) uniform image2D o_color;
layout(set = 1, binding = 1, rgba32f) uniform image2D o_normal;
layout(set = 1, binding = 2, rgba32f) uniform image2D o_position;

#include "trace.glsl"

#include "sampler/independent.glsl"
#include "bsdf/diffuse.glsl"
#include "camera.glsl"
#include "emitter.glsl"

// float mis_weight(float pdf_a, float pdf_b){
//     if (pdf_a > 0.){
//         return pdf_a / (pdf_a + pdf_b);
//     }else{
//         return 0.;
//     }
// }
float mis_weight(float pdf_a, float pdf_b){
    float a2 = pdf_a * pdf_a;
    if (pdf_a > 0.){
        return a2 / (pdf_b * pdf_b + a2);
    }else{
        return 0.;
    }
}

uint pixel_idx = (gl_LaunchIDEXT.y * gl_LaunchSizeEXT.x + gl_LaunchIDEXT.x);

void main(){
    const vec2 pos = vec2(gl_LaunchIDEXT.xy);
    
    uint idx = uint(gl_LaunchSizeEXT.x * pos.y + pos.x);

    SampleGenerator sample_generator = sample_generator(push_constant.seed, idx);
    
    vec2 sample_pos = pos + next_2d(sample_generator);
    vec2 adjusted_pos = sample_pos / vec2(gl_LaunchSizeEXT.xy);

    Camera camera = cameras[push_constant.camera];
    
    Ray ray = sample_ray(camera, adjusted_pos);

    vec3 L = vec3(0.);
    vec3 f = vec3(1.);
    uint depth = 0;
    float prev_bsdf_pdf = 1.;
    
    SurfaceInteraction si;
    
    while (depth < push_constant.max_depth){
        si = ray_intersect(ray);

        if (!si.valid){
            // TODO: Constant emission
            break;
        }

        //===========================================================
        // Storing normal and position:
        //===========================================================
        if (depth == 0){
            imageStore(o_normal, ivec2(pos), vec4(si.n, 1.));
            imageStore(o_position, ivec2(pos), vec4(si.p, 1.));
        }

        //===========================================================
        // BSDF Sampling:
        //===========================================================
        BSDFSample bs;
        vec3 bsdf_value;
        sample_bsdf(si, next_1d(sample_generator), next_2d(sample_generator), bs, bsdf_value);
        
        //===========================================================
        // Direct Emission:
        //===========================================================

        float em_pdf = depth == 0?0.:pdf_emitter_direction(si);
        
        float mis_bsdf = mis_weight(prev_bsdf_pdf, em_pdf);

        vec3 direct_emission = eval_emitter(si);
        
        L += f * direct_emission * mis_bsdf;

        //===========================================================
        // Emitter Sampling:
        //===========================================================
        DirectionSample ds;
        vec3 em_weight;
        sample_emitter_direction(si, next_2d(sample_generator), ds, em_weight);

        vec3 em_bsdf_weight;
        float em_bsdf_pdf;
        eval_pdf(si, to_local(si, ds.d), em_bsdf_weight, em_bsdf_pdf);

        float mis_em = mis_weight(ds.pdf, em_bsdf_pdf);

        L += f * em_weight * em_bsdf_weight * mis_em;
        
        //===========================================================
        // Update Loop Variables:
        //===========================================================
        
        f *= bsdf_value;
        ray = spawn_ray(si, to_world(si, bs.wo));
        prev_bsdf_pdf = bs.pdf;
        
        //===========================================================
        // Russian Roulette:
        //===========================================================
        float f_max = max(f.r, max(f.g, f.b));
        float rr_prop = f_max;

        if (depth < push_constant.rr_depth){
            rr_prop = 1.;
        }
        f *= 1. / rr_prop;
        bool rr_continue = next_1d(sample_generator) < rr_prop;
        if (!rr_continue){
            break;
        }

        depth += 1;

    }
    imageStore(o_color, ivec2(pos), vec4(L, 0.));
}

