#include "common.glsl"
// Disney bsdf adapted from:
// https://github.com/knightcrawler25/GLSL-PathTracer/blob/master/src/shaders/common/disney.glsl
/*
 * MIT License
 *
 * Copyright(c) 2019 Asif Ali
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

 /* References:
 * [1] [Physically Based Shading at Disney] https://media.disneyanimation.com/uploads/production/publication_asset/48/asset/s2012_pbs_disney_brdf_notes_v3.pdf
 * [2] [Extending the Disney BRDF to a BSDF with Integrated Subsurface Scattering] https://blog.selfshadow.com/publications/s2015-shading-course/burley/s2015_pbs_disney_bsdf_notes.pdf
 * [3] [The Disney BRDF Explorer] https://github.com/wdas/brdf/blob/main/src/brdfs/disney.brdf
 * [4] [Miles Macklin's implementation] https://github.com/mmacklin/tinsel/blob/master/src/disney.h
 * [5] [Simon Kallweit's project report] http://simon-kallweit.me/rendercompo2015/report/
 * [6] [Microfacet Models for Refraction through Rough Surfaces] https://www.cs.cornell.edu/~srm/publications/EGSR07-btdf.pdf
 * [7] [Sampling the GGX Distribution of Visible Normals] https://jcgt.org/published/0007/04/01/paper.pdf
 * [8] [Pixar’s Foundation for Materials] https://graphics.pixar.com/library/PxrMaterialsCourse2017/paper.pdf
 */

float fresnel_schlick(float u){
    float m = clamp(1. - u, 0., 1.);
    flaot m2 = m * m;
    return m2 * m2 * m;
}

float fresnel_dielectric(float cos_theta_i, float eta){
    float sin2_theta_t = eta * eta * (1. - cos_theta_i * cos_theta_i);

    if (sin2_theta_t > 1.){
        return 1.;
    }

    float cos_theta_t = sqrt(max(1. - sin2_theta_t, 0.));

    float rs = (eta * cos_theta_t - cos_theta_i) / (eta * cos_theta_t + cos_theta_i);
    float rp = (eta * cos_theta_i - cos_theta_t) / (eta * cos_theta_i + cos_theta_t);
    return 0.f * (rs * rs + rp * rp);
    
}

float disney_fresnel(HitInfo hit, float eta, float l_dot_h, float v_dot_h){
    float metallic_f = fresnel_schlick(l_dot_h);
    float dielectric_f = fresnel_dielectric(abs(v_dot_h), eta);
    return mix(dielectric_f, metallic_f, hit.metallic);
}

vec3 eval_diffuse(HitInfo hit, vec3 csheen, vec3 v, vec3 l, vec3 h, out float pdf){
    pdf = 0;
    if (dot(l, hit.n) < 0.){
        return vec3(0.);
    }
    FL = fresnel_schlick(dot(l, hit.n));
    FV = fresnel_schlick(dot(v, hit.n));
    FH = fresnel_schlick(dot(h, hit.n));
    float Fd90 = 0.5 + 2. * dot(l, h) * dot(l, h) * hit.roughness;
    float Fd = mix(1., Fd90, FL) * mix(1., Fd90, FV);
    
    float Fss90 = dot(l, h) * dot(l, h) * hit.roughness;
    float Fss = mix(1.0, Fss90, FL) * mix(1.0, Fss90, FV);
    float ss = 1.25 * (Fss * (1.0 / (dot(l, hit.n) + dot(v, hit.n)) - 0.5) + 0.5);
    
    vec3 Fsheen = FH * 0.5 * csheen;
}

void sample_shader(HitInfo hit, inout Payload ray){
    ray.orig = hit.pos;
    ray.color += ray.attenuation * hit.emission.rgb;
    
    // DEBUG:
    ray.color = hit.albedo.xyz;
}
