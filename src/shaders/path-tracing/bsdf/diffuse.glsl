#ifndef DIFFUSE_BSDF_GLSL
#define DIFFUSE_BSDF_GLSL

#include "interaction.glsl"
#include "warp.glsl"
#include "texture.glsl"

// Sample an outgoing direction wo and evaluate the bsdf for that direction.
//
// value: The BSDF value f(wi, wo) divided by the probability p(wo)
//        (multiplied by the cosinus foreshortening term cos_theta_o for non-delta components).
//      

void sample_bsdf(
    in SurfaceInteraction si, 
    in float sample1, 
    in vec2 sample2, 
    out BSDFSample bs, 
    out vec3 value){

    float cos_theta_i = cos_theta(si.wi);
    
    bs.wo = square_to_cosine_hemisphere(sample2);
    bs.pdf = square_to_cosine_hemisphere_pdf(bs.wo);
    
    value = eval_texture(si.material.base_color, si.uv);
}


// Evaluate the bsdf including the cosinus foreshortening term.
// f(wi, wo) * cos_theta_o
vec3 eval_bsdf(in SurfaceInteraction si, in vec3 wo){
    float cos_theta_i = cos_theta(si.wi);
    float cos_theta_o = cos_theta(wo);

    if (cos_theta_i > 0. && cos_theta_o > 0.){
        return eval_texture(si.material.base_color, si.uv) / PI * cos_theta_o;
    }else{
        return vec3(0.);
    }
}

// Calculate the probability of sampling a direction wo when using the function sample_bsdf.
float bsdf_pdf(in SurfaceInteraction si, in vec3 wo){
    float cos_theta_i = cos_theta(si.wi);
    float cos_theta_o = cos_theta(wo);

    if (cos_theta_i > 0. && cos_theta_o > 0.){
        return square_to_cosine_hemisphere_pdf(wo);
    }else{
        return 0.;
    }
}

// Combine eval and pdf
void bsdf_eval_pdf(in SurfaceInteraction si, in vec3 wo, out vec3 value, out float pdf){
    float cos_theta_i = cos_theta(si.wi);
    float cos_theta_o = cos_theta(wo);

    if (cos_theta_i > 0. && cos_theta_o > 0.){
        pdf = square_to_cosine_hemisphere_pdf(wo);
        value = eval_texture(si.material.base_color, si.uv) / PI * cos_theta_o;
    }else{
        pdf = 0.;
        value = vec3(0.);
    }
}

#endif //DIFFUSE_BSDF_GLSL
