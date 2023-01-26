#ifndef RESTIR_COMMON_GLSL
#define RESTIR_COMMON_GLSL

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
vec3 sample_outgoing(in SurfaceInteraction si, inout SampleGenerator sample_generator){
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
        bsdf_eval_pdf(si, to_local(si, ds.d), em_bsdf_weight, em_bsdf_pdf);

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
    return L;
}
#endif //RESTIR_COMMON_GLSL
