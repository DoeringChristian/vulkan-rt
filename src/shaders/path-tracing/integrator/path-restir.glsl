#ifndef PATH_GLSL
#define PATH_GLSL

#include "emitter.glsl"
#include "camera.glsl"
#include "interaction.glsl"
#include "restir_reservoir.glsl"
#include "spectrum.glsl"
#include "warp.glsl"

float mis_weight(float pdf_a, float pdf_b){
    float a2 = pdf_a * pdf_a;
    if (pdf_a > 0.){
        return a2 / (pdf_b * pdf_b + a2);
    }else{
        return 0.;
    }
}

// Sample outgoing radiance at point si.p towards si.wi
// Returns: L_o(si.p, si.wi)
vec3 sample_outgoing(in SurfaceInteraction si){
    vec3 L = vec3(0.);
    vec3 f = vec3(1.);
    uint depth = 0;
    float prev_bsdf_pdf = 1.;

    Ray ray;
    
    while (depth < push_constant.max_depth){
        if (depth > 0){
            si = ray_intersect(ray);
        }

        if (!si.valid){
            // TODO: Constant emission
            break;
        }

        //===========================================================
        // BSDF Sampling:
        //===========================================================
        BSDFSample bs;
        vec3 bsdf_value;
        sample_bsdf(si, next_1d(), next_2d(), bs, bsdf_value);
        
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
        sample_emitter_direction(si, next_2d(), ds, em_weight);

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
        bool rr_continue = next_float() < rr_prop;
        if (!rr_continue){
            break;
        }

        depth += 1;
    }
    return L;
}

void render(uvec2 size, uvec2 pos){
    uint pixel = uint(size.x * pos.y + pos.x);

    pcg_init(sample_tea_32(push_constant.seed, pixel));
    
    vec2 sample_pos = vec2(pos) + next_2d();
    vec2 adjusted_pos = sample_pos / vec2(size);

    Ray ray = sample_ray(adjusted_pos);

    vec3 L = sample_ray(ray);

    SurfaceInteraction si = ray_intersect(ray);

    BSDFSample bs;
    vec3 bsdf_value;
    sample_bsdf(si_v, next_2d(), bs, bsdf_value);

    //imageStore(o_color, ivec2(pos), vec4(si_v.n, 0.));

    ray = spawn_ray(to_world(si_v, bs.wo));
    si_s = ray_intersect(ray);
        
    vec3 Li = sample_outgoing(si_s); // radiance from sample point x_s towards visible point x_v

    vec3 Lo = Li * bsdf_value; // bsdf_value = f(wo, wi) * cos_theta_o

    float p_hat = luminance(Lo);

    RestirSample S = RestirSample(si_v.p, si_v.n, si_s.p, si_s.n, Li); // instead of retreiving from buffer we sample in this shader

    //===========================================================
    // Temporal Resampling (Algorithm 3):
    //===========================================================

    RestirReservoir R = temporal_reservoir[pixel]; // l.3

    float w = p_hat / bs.pdf; // l.4

    update(R, S, w); // l.5

    R.W = R.w / (R.M); // l.6 TODO: Multiply by p^\hat(R.z)

    temporal_reservoir[pixel] = R; // l.7
    
    //===========================================================
    // Temporal Resampling (Algorithm 4):
    //===========================================================
    uint max_iterations = 9;

    RestirReservoir R_s = spatial_reservoir[pixel]; // l.2

    for (uint s = 0; s < max_iterations; s++){ // l.4
        vec2 pos_n = pos + uvec2(square_to_uniform_disk_concentric(next_2d()) * float(size.x) * 0.1); // l.5
        uint q_n = pos_n.y * size.x + pos_n.y; // l.5

        // TODO: similarity // l.6-8

        RestirReservoir R_n = temporal_reservoir[q_n]; // l.9

        

        
        
    }
    
    
}

#endif // PATH_GLSL
