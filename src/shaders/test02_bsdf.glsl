
#include "common.glsl"

// From LearnOpenGL

float DistributionGGX(vec3 N, vec3 H, float roughness)
{
    float a      = roughness*roughness;
    float a2     = a*a;
    float NdotH  = max(dot(N, H), 0.0);
    float NdotH2 = NdotH*NdotH;
	
    float num   = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = M_PI * denom * denom;
	
    return num / denom;
}

float GeometrySchlickGGX(float NdotV, float roughness)
{
    float r = (roughness + 1.0);
    float k = (r*r) / 8.0;

    float num   = NdotV;
    float denom = NdotV * (1.0 - k) + k;
	
    return num / denom;
}
float GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness)
{
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2  = GeometrySchlickGGX(NdotV, roughness);
    float ggx1  = GeometrySchlickGGX(NdotL, roughness);
	
    return ggx1 * ggx2;
}
vec3 fresnelSchlick(float cosTheta, vec3 F0)
{
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}
float fresnelSchlickReflectAmount(float cosTheta, float n1, float n2, float f0){
    float r0 = (n1-n2)/(n1+n2);
    r0 *= r0;
    if (n1 > n2){
        float n = n1/n2;
        float sinT2 = n*n*(1.-cosTheta*cosTheta);
        if(sinT2 > 1.){
            return 1.;
        }
        cosTheta = sqrt(1. - sinT2);
    }
    float x = 1.0 - cosTheta;
    float ret = r0+(1.-r0)*x*x*x*x*x;
    return mix(f0, 1., ret);
}

struct Sample{
    vec3 dir;
    vec3 bsdf;
};

// from https://agraphicsguy.wordpress.com/2015/11/01/sampling-microfacet-brdf/
vec4 sample_DistributionGGX(float roughness, vec3 n, vec3 seed){
    float a = roughness * roughness;
    float a2 = a * a;

    vec2 e = rand2(seed);

    float theta = acos(sqrt((1. - e.x)/((a2 - 1.) * e.x + 1.)));
    float phi = 2. * M_PI * e.y;

    vec3 n_ndf = vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );

    n_ndf = allign_hemisphere(n_ndf, n);
    return vec4(normalize(n_ndf), 1./(2. * M_PI));
}

Sample generate_sample(vec3 n, vec3 wo, InterMaterial mat, vec3 seed){
    float metallic = mat.mr.x;
    float roughness = mat.mr.y;
    vec3 albedo = mat.albedo.xyz;

    //vec4 ndf_sample = sample_DistributionGGX(roughness, n, seed);

    
    float n1 = 1.;
    float n2 = 1.;
    if (dot(n, wo) > 0){
        // the light is reflecting/refacting from the ari.
        n1 = 1.;
        //n2 = mat.ior;
        n2 = 1.04;
    }
    else{
        //n1 = mat.ior;
        n1 = 1.04;
        n2 = 1.;
    }
    
    vec4 ndf_sample = sample_DistributionGGX(roughness, n, seed - vec3(M_PI));
    // m is the microfacet normal
    vec3 m = ndf_sample.xyz;
    
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, mat.albedo.xyz, metallic);

    vec3 F = fresnelSchlick(max(0., dot(m, wo)), F0);
    float F_avg = (F.x+F.y+F.z)/3.;
    //F = vec3(0.);

    float kS = F_avg;
    float kD = 1. - F_avg;

    if (rand(seed + vec3(M_PI)) < kS){
        // Specular case

        vec3 wi = reflect(-wo, m);
        float wi_dot_n = max(dot(m, wi), 0.);
        float G = GeometrySmith(n, wo, wi, roughness);

        vec3 numerator = G * vec3(1.);
        float denominator = 4. * max(dot(m, wo), 0.) * max(dot(m, wi), 0.) + 0.001;
        vec3 specular = numerator/denominator;
        vec3 fr = specular * F;
        return Sample(wi, fr * wi_dot_n * (2 * M_PI) / kS);
    }
    else{
        // Diffuse case
        vec3 wi = allign_hemisphere(uniform_hemisphere(seed), n);
        float wi_dot_n = max(dot(n, wi), 0.);

        vec3 fr = (1. - metallic) * albedo;

        return Sample(wi, fr * wi_dot_n * (2. * M_PI));
    }
}
