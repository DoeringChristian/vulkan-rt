
#include "common.glsl"

// From LearnOpenGL

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
    return vec4(n_ndf, 1./(2. * M_PI));
}

Sample generate_sample(vec3 n, vec3 wo, InterMaterial mat, vec3 seed){
    float roughness = mat.mr.y;
    float metallic = mat.mr.x;
    vec3 albedo = mat.albedo.xyz;

    vec4 ndf_sample = sample_DistributionGGX(roughness, n, seed);
    vec3 n_ndf = ndf_sample.xyz;
    
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
    
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, mat.albedo.xyz, mat.mr.x);

    vec3 F = fresnelSchlick(max(0., dot(n_ndf, wo)), F0);
    

    if (rand(seed + vec3(M_PI)) < length(F)){
        // Specular case
        vec3 wi = reflect(-wo, n_ndf);
        float wi_dot_n = max(dot(n_ndf, wi), 0.);
        float G = GeometrySmith(n, wo, wi, roughness);

        vec3 numerator = G * F;
        float denominator = 4. * max(dot(n, wo), 0.) * max(dot(n, wi), 0.) + 0.001;
        vec3 specular = numerator/denominator;
        vec3 fr = specular;
        return Sample(wi, fr * wi_dot_n / ndf_sample.w / length(F));
    }
    else{
        // Diffuse case
        vec3 wi = uniform_hemisphere(n, seed);
        float wi_dot_n = max(dot(n_ndf, wi), 0.);
        
        vec3 kD = vec3(1.) - F;
        kD *= 1. - metallic;
        vec3 fr = kD * albedo / M_PI;
        
        return Sample(wi, fr * wi_dot_n / ndf_sample.w / length(F));
    }
}
