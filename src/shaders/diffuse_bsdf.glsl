#ifndef DIFFUSE_BSDF_GLSL
#define DIFFUSE_BSDF_GLSL

#include "bindings.glsl"
#include "utils.glsl"
#include "warp.glsl"

void sample_bsdf(
    in SurfaceInteraction si, 
    in float sample1, 
    in vec2 sample2, 
    out BSDFSample bs, 
    out vec3 value){

    float cos_theta_i = to_local(si, si.wi).z;
    
    bs.wo = square_to_cosine_hemisphere(sample2);
    bs.pdf = square_to_cosine_hemisphere_pdf(bs.wo);
    
    value = eval_texture(si.material.base_color, si);
}

vec3 eval(in SurfaceInteraction si, in vec3 wo){
    float cos_theta_i = to_local(si, si.wi).z;
    float cos_theta_o = to_local(si, wo).z;

    if (cos_theta_i > 0. && cos_theta_o > 0.){
        return eval_texture(si.material.base_color, si);
    }else{
        return vec3(0.);
    }
}

float pdf(in SurfaceInteraction si, in vec3 wo){
    float cos_theta_i = to_local(si, si.wi).z;
    float cos_theta_o = to_local(si, wo).z;

    if (cos_theta_i > 0. && cos_theta_o > 0.){
        return square_to_cosine_hemisphere_pdf(wo);
    }else{
        return 0.;
    }
}

void eval_pdf(in SurfaceInteraction si, in vec3 wo, out vec3 value, out float pdf){
    float cos_theta_i = to_local(si, si.wi).z;
    float cos_theta_o = to_local(si, wo).z;

    if (cos_theta_i > 0. && cos_theta_o > 0.){
        pdf = square_to_cosine_hemisphere_pdf(wo);
        value = eval_texture(si.material.base_color, si);
    }else{
        pdf = 0.;
        value = vec3(0.);
    }
}

#endif //DIFFUSE_BSDF_GLSL
