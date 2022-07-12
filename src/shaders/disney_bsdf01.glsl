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
#include "common.glsl"
#include "sampling.glsl"
#include "math.glsl"

float Luminance(vec3 c)
{
    return 0.212671 * c.x + 0.715160 * c.y + 0.072169 * c.z;
}

vec3 ToWorld(vec3 X, vec3 Y, vec3 Z, vec3 V)
{
    return V.x * X + V.y * Y + V.z * Z;
}

vec3 ToLocal(vec3 X, vec3 Y, vec3 Z, vec3 V)
{
    return vec3(dot(V, X), dot(V, Y), dot(V, Z));
}

float DisneyFresnel(Material mat, float eta, float LDotH, float VDotH)
{
    float metallicFresnel = SchlickFresnel(LDotH);
    float dielectricFresnel = DielectricFresnel(abs(VDotH), eta);
    return mix(dielectricFresnel, metallicFresnel, mat.metallic);
}

vec3 EvalDiffuse(Material mat, vec3 Csheen, vec3 V, vec3 L, vec3 H, out float pdf)
{
    pdf = 0.0;
    if (L.z <= 0.0)
        return vec3(0.0);

    // Diffuse
    float FL = SchlickFresnel(L.z);
    float FV = SchlickFresnel(V.z);
    float FH = SchlickFresnel(dot(L, H));
    float Fd90 = 0.5 + 2.0 * dot(L, H) * dot(L, H) * mat.roughness;
    float Fd = mix(1.0, Fd90, FL) * mix(1.0, Fd90, FV);

    // Fake Subsurface TODO: Replace with volumetric scattering
    float Fss90 = dot(L, H) * dot(L, H) * mat.roughness;
    float Fss = mix(1.0, Fss90, FL) * mix(1.0, Fss90, FV);
    float ss = 1.25 * (Fss * (1.0 / (L.z + V.z) - 0.5) + 0.5);

    // Sheen
    vec3 Fsheen = FH * mat.sheen * Csheen;

    pdf = L.z * INV_PI;
    return (1.0 - mat.metallic) * (1.0 - mat.transmission) * (INV_PI * mix(Fd, ss, mat.subsurface) * mat.albedo + Fsheen);
}

vec3 EvalSpecReflection(Material mat, float eta, vec3 specCol, vec3 V, vec3 L, vec3 H, out float pdf)
{
    pdf = 0.0;
    if (L.z <= 0.0)
        return vec3(0.0);

    float FM = DisneyFresnel(mat, eta, dot(L, H), dot(V, H));
    vec3 F = mix(specCol, vec3(1.0), FM);
    float D = GTR2Aniso(H.z, H.x, H.y, mat.ax, mat.ay);
    float G1 = SmithGAniso(abs(V.z), V.x, V.y, mat.ax, mat.ay);
    float G2 = G1 * SmithGAniso(abs(L.z), L.x, L.y, mat.ax, mat.ay);

    pdf = G1 * D / (4.0 * V.z);
    return F * D * G2 / (4.0 * L.z * V.z);
}

vec3 EvalSpecRefraction(Material mat, float eta, vec3 V, vec3 L, vec3 H, out float pdf)
{
    pdf = 0.0;
    if (L.z >= 0.0)
        return vec3(0.0);

    float F = DielectricFresnel(abs(dot(V, H)), eta);
    float D = GTR2Aniso(H.z, H.x, H.y, mat.ax, mat.ay);
    float G1 = SmithGAniso(abs(V.z), V.x, V.y, mat.ax, mat.ay);
    float G2 = G1 * SmithGAniso(abs(L.z), L.x, L.y, mat.ax, mat.ay);
    float denom = dot(L, H) + dot(V, H) * eta;
    denom *= denom;
    float eta2 = eta * eta;
    float jacobian = abs(dot(L, H)) / denom;

    pdf = G1 * max(0.0, dot(V, H)) * D * jacobian / V.z;

    return pow(mat.albedo, vec3(0.5)) * (1.0 - mat.metallic) * mat.transmission * (1.0 - F) * D * G2 * abs(dot(V, H)) * jacobian * eta2 / abs(L.z * V.z);
}

vec3 EvalClearcoat(Material mat, vec3 V, vec3 L, vec3 H, out float pdf)
{
    pdf = 0.0;
    if (L.z <= 0.0)
        return vec3(0.0);

    float FH = DielectricFresnel(dot(V, H), 1.0 / 1.5);
    float F = mix(0.04, 1.0, FH);
    float D = GTR1(H.z, mat.clearcoatRoughness);
    float G = SmithG(L.z, 0.25)
        * SmithG(V.z, 0.25);
    float jacobian = 1.0 / (4.0 * dot(V, H));

    pdf = D * H.z * jacobian;
    return vec3(0.25) * mat.clearcoat * F * D * G / (4.0 * L.z * V.z);
}

void GetSpecColor(Material mat, float eta, out vec3 specCol, out vec3 sheenCol)
{
    float lum = Luminance(mat.albedo);
    vec3 ctint = lum > 0.0 ? mat.albedo / lum : vec3(1.0f);
    float F0 = (1.0 - eta) / (1.0 + eta);
    specCol = mix(F0 * F0 * mix(vec3(1.0), ctint, mat.specularTint), mat.albedo, mat.metallic);
    sheenCol = mix(vec3(1.0), ctint, mat.sheenTint);
}

void GetLobeProbabilities(Material mat, float eta, vec3 specCol, float approxFresnel, out float diffuseWt, out float specReflectWt, out float specRefractWt, out float clearcoatWt)
{
    diffuseWt = Luminance(mat.albedo) * (1.0 - mat.metallic) * (1.0 - mat.transmission);
    specReflectWt = Luminance(mix(specCol, vec3(1.0), approxFresnel));
    specRefractWt = (1.0 - approxFresnel) * (1.0 - mat.metallic) * mat.transmission * Luminance(mat.albedo);
    clearcoatWt = 0.25 * mat.clearcoat * (1.0 - mat.metallic);
    float totalWt = diffuseWt + specReflectWt + specRefractWt + clearcoatWt;

    diffuseWt /= totalWt;
    specReflectWt /= totalWt;
    specRefractWt /= totalWt;
    clearcoatWt /= totalWt;
}

vec3 DisneySample(Material mat, float eta, vec3 V, vec3 N, out vec3 L, out float pdf)
{
    pdf = 0.0;
    vec3 f = vec3(0.0);

    float r1 = randf();
    float r2 = randf();

    // TODO: Tangent and bitangent should be calculated from mesh (provided, the mesh has proper uvs)
    vec3 T, B;
    Onb(N, T, B);
    V = ToLocal(T, B, N, V); // NDotL = L.z; NDotV = V.z; NDotH = H.z

    // Specular and sheen color
    vec3 specCol, sheenCol;
    GetSpecColor(mat, eta, specCol, sheenCol);

    // Lobe weights
    float diffuseWt, specReflectWt, specRefractWt, clearcoatWt;
    // Note: Fresnel is approx and based on N and not H since H isn't available at this stage.
    float approxFresnel = DisneyFresnel(mat, eta, V.z, V.z);
    GetLobeProbabilities(mat, eta, specCol, approxFresnel, diffuseWt, specReflectWt, specRefractWt, clearcoatWt);

    // CDF for picking a lobe
    float cdf[4];
    cdf[0] = diffuseWt;
    cdf[1] = cdf[0] + clearcoatWt;
    cdf[2] = cdf[1] + specReflectWt;
    cdf[3] = cdf[2] + specRefractWt;

    if (r1 < cdf[0]) // Diffuse Reflection Lobe
    {
        r1 /= cdf[0];
        L = CosineSampleHemisphere(r1, r2);

        vec3 H = normalize(L + V);

        f = EvalDiffuse(mat, sheenCol, V, L, H, pdf);
        pdf *= diffuseWt;
    }
    else if (r1 < cdf[1]) // Clearcoat Lobe
    {
        r1 = (r1 - cdf[0]) / (cdf[1] - cdf[0]);

        vec3 H = SampleGTR1(mat.clearcoatRoughness, r1, r2);

        if (H.z < 0.0)
            H = -H;

        L = normalize(reflect(-V, H));

        f = EvalClearcoat(mat, V, L, H, pdf);
        pdf *= clearcoatWt;
    }
    else  // Specular Reflection/Refraction Lobes
    {
        r1 = (r1 - cdf[1]) / (1.0 - cdf[1]);
        vec3 H = SampleGGXVNDF(V, mat.ax, mat.ay, r1, r2);

        if (H.z < 0.0)
            H = -H;

        // TODO: Refactor into metallic BRDF and specular BSDF
        float fresnel = DisneyFresnel(mat, eta, dot(L, H), dot(V, H));
        float F = 1.0 - ((1.0 - fresnel) * mat.transmission * (1.0 - mat.metallic));

        if (randf() < F)
        {
            L = normalize(reflect(-V, H));

            f = EvalSpecReflection(mat, eta, specCol, V, L, H, pdf);
            pdf *= F;
        }
        else
        {
            L = normalize(refract(-V, H, eta));

            f = EvalSpecRefraction(mat, eta, V, L, H, pdf);
            pdf *= 1.0 - F;
        }

        pdf *= specReflectWt + specRefractWt;
    }

    L = ToWorld(T, B, N, L);
    return f * abs(dot(N, L));
}
vec3 DisneyEval(Material mat, float eta, vec3 V, vec3 N, vec3 L, out float bsdfPdf){
    bsdfPdf = 0.0;
    vec3 f = vec3(0.0);

    // TODO: Tangent and bitangent should be calculated from mesh (provided, the mesh has proper uvs)
    vec3 T, B;
    Onb(N, T, B);
    V = ToLocal(T, B, N, V); // NDotL = L.z; NDotV = V.z; NDotH = H.z
    L = ToLocal(T, B, N, L);

    vec3 H;
    if (L.z > 0.0)
        H = normalize(L + V);
    else
        H = normalize(L + V * eta);

    if (H.z < 0.0)
        H = -H;

    // Specular and sheen color
    vec3 specCol, sheenCol;
    GetSpecColor(mat, eta, specCol, sheenCol);

    // Lobe weights
    float diffuseWt, specReflectWt, specRefractWt, clearcoatWt;
    float fresnel = DisneyFresnel(mat, eta, dot(L, H), dot(V, H));
    GetLobeProbabilities(mat, eta, specCol, fresnel, diffuseWt, specReflectWt, specRefractWt, clearcoatWt);

    float pdf;

    // Diffuse
    if (diffuseWt > 0.0 && L.z > 0.0)
    {
        f += EvalDiffuse(mat, sheenCol, V, L, H, pdf);
        bsdfPdf += pdf * diffuseWt;
    }

    // Specular Reflection
    if (specReflectWt > 0.0 && L.z > 0.0 && V.z > 0.0)
    {
        f += EvalSpecReflection(mat, eta, specCol, V, L, H, pdf);
        bsdfPdf += pdf * specReflectWt;
    }

    // Specular Refraction
    if (specRefractWt > 0.0 && L.z < 0.0)
    {
        f += EvalSpecRefraction(mat, eta, V, L, H, pdf);
        bsdfPdf += pdf * specRefractWt;
    }

    // Clearcoat
    if (clearcoatWt > 0.0 && L.z > 0.0 && V.z > 0.0)
    {
        f += EvalClearcoat(mat, V, L, H, pdf);
        bsdfPdf += pdf * clearcoatWt;
    }

    return f * abs(L.z);
}

// Do not modify throughput or radiance of the ray
void sample_shader(
    in HitInfo hit, 
    in Material mat, 
    // Medium from the ray TODO: rename
    inout Medium rmed,
    inout vec3 orig,
    inout vec3 dir,
    out vec3 radiance, 
    out vec3 f,
    out float pdf
){
    // Initialisation of out variables
    radiance = vec3(0.);
    f = vec3(1.);
    pdf = 1.;
    
    bool inside = dot(hit.g, hit.wo) < 0.;

    Medium med;
    
    // Select medium through which the ray has traveled.
    if(inside){
        med = mat.med;
    } else{
        med = rmed;
    }

    // Do medium scattering
    float scatterDist = min(-log(randf()/med.density) * med.density, hit.dist);
    bool scattered = scatterDist < hit.dist;

    // Absorbtion
    f *= exp(-(1. - med.color) * hit.dist * med.density);

    if(scattered){
        // Set origin of scatterd ray
        orig = orig + scatterDist * dir;
        f *= med.color;
        
        vec3 L = SampleHG(-dir, med.anisotropic, randf(), randf());
        pdf *= PhaseHG(dot(hit.wo, L), med.anisotropic);
        dir = L;
    }else{
        orig = hit.pos;
        radiance += mat.emission.rgb;
        
        float eta;
        vec3 ffnormal;
        if (dot(hit.g, hit.wo) < 0.){
            ffnormal = -hit.n;
            eta = mat.ior/1.;
        } else{
            ffnormal = hit.n;
            eta = 1. / mat.ior;
        }

        vec3 L = vec3(dir);
        f *= DisneySample(mat, eta, hit.wo, ffnormal, L, pdf);

        dir = normalize(L);


        // select medium through which the ray travels
        inside = dot(dir, hit.g) < 0.;
        if (inside){
            rmed = mat.med;
        }
        
    }
}

void eval_shader(in HitInfo hit, in Material mat, in vec3 L, out vec3 f, out float pdf){

    pdf = 1.;
    f = vec3(1.);
    
    float eta;
    vec3 ffnormal;
    if (dot(hit.g, hit.wo) < 0.){
        ffnormal = -hit.n;
        eta = mat.ior/1.;
    } else{
        ffnormal = hit.n;
        eta = 1. / mat.ior;
    }

    f *= DisneyEval(mat, eta, hit.wo, ffnormal, L, pdf);
}
