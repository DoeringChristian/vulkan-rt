#ifndef PATH_GLSL
#define PATH_GLSL

#include "interaction.glsl"
#include "sensor/perspective.glsl"

float mis_weight(float pdf_a, float pdf_b){
    float a2 = pdf_a * pdf_a;
    if (pdf_a > 0){
        return a2 / (pdf_a * pdf_b + a2);
    }else{
        return 0;
    }
}

void render(uvec2 size, uvec2 pos){
    uint idx = uint(size.x * pos.y + pos.x);

    pcg_init(sample_tea_32(push_constant.seed, idx));

    Ray ray = sample_ray(vec2(pos), vec2(size));

    vec3 L = vec3(0.);
    vec3 f = vec3(1.);
    uint depth = 0;
    
    SurfaceInteraction si;
    
    while (depth < push_constant.max_depth){
        si = ray_intersect(ray);

        finalize_surface_interaction(si, ray);
        // DEBUG:

        BSDFSample bs;
        vec3 bsdf_value;
        sample_bsdf(si, next_1d(), next_2d(), bs, bsdf_value);


        L += f * eval_texture(si.material.emission, si);
        f *= bsdf_value;

        ray = spawn_ray(si, to_world(si, bs.wo));

        //===========================================================
        // Throughput Russian Roulette:
        //===========================================================
        float f_max = max(f.r, max(f.g, f.b));
        float rr_prop = f_max;

        if (depth < push_constant.rr_depth){
            rr_prop = 1.;
        }
        f *= 1. / rr_prop;
        bool rr_continue = next_float() < rr_prop;
        if (!rr_continue){
            break;
        }

        depth += 1;

        // DEBUG:
        //L = vec3(si.uv, 0.);
        //L = eval_texture(si.material.base_color, si);
        //break;
    }
    
    imageStore(image[0], ivec2(pos), vec4(L, 1.));
}

#endif // PATH_GLSL
