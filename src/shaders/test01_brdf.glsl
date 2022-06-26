
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
    float prop;
    vec3 n_ndf;
    bool specular;
};

Evaluation eval(vec3 n, vec3 wo, Sample s, InterMaterial mat){
    float roughness = mat.mr.y;
    float metallic = mat.mr.x;
    
    vec3 albedo = mat.albedo.xyz;
    
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    // The halfway vector is the normal sampled by the NDF
    vec3 n_ndf = s.n_ndf;
    //float NDF = DistributionGGX(n, h, roughness);
    float G = GeometrySmith(n, wo, s.dir, roughness);
    vec3 F = fresnelSchlick(max(dot(n_ndf, wo), 0.), F0);

    vec3 fr = vec3(0.);
    if (s.specular){
        vec3 numerator = vec3(G) * F;
        float denominator = 4. * max(dot(n, wo), 0.) * max(dot(n, s.dir), 0.) + 0.0001;
        vec3 specular = numerator/denominator;
        fr = specular;
    }
    else{
        //vec3 kD = vec3(1.) - F;
        vec3 kD = vec3(1.);
        kD *= 1. - metallic;
        fr = albedo / M_PI;
    }
    

    /*
    vec3 kS = F;
    vec3 kD = vec3(1.) - kS;
    kD *= 1. - metallic;

    vec3 numerator = G * F;
    float denominator = 4. * max(dot(n, wo), 0.) * max(dot(n, s.dir), 0.) + 0.0001;
    vec3 specular = numerator / denominator;

    float won = max(dot(n, wo), 0.);
    float win = max(dot(n, s.dir), 0.);
    
    vec3 fr = (kD * albedo / M_PI + specular);
    */
    float win = max(dot(n, s.dir), 0.);

    return Evaluation(fr * win / s.prop, s.dir);
}

vec3 sample_DistributionGGX(float roughness, vec3 n, vec3 seed){
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
    return n_ndf;
}

// from https://agraphicsguy.wordpress.com/2015/11/01/sampling-microfacet-brdf/
Sample generate_sample(vec3 n, vec3 wo, InterMaterial mat, vec3 seed){
    float roughness = mat.mr.y;

    vec3 n_ndf = sample_DistributionGGX(roughness, n, seed);

    float n1 = 0.;
    float n2 = 0.;
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
    //float F0_sqrt = (n1 - n2) / (n2 - n1);
    //vec3 F0 = vec3(F0_sqrt * F0_sqrt);
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, mat.albedo.xyz, mat.mr.x);

    //float F = fresnelSchlickReflectAmount(max(0., dot(n_ndf, wo)), n1, n2, 0.5);
    vec3 F = fresnelSchlick(max(0., dot(n_ndf, wo)), F0);

    vec3 dir = vec3(0.);
    bool specular = false;

    if (rand(seed + vec3(M_PI)) < length(F)){
        // Reflect
        dir = reflect(-wo, n_ndf);
        specular = true;
    }
    else{
        // Refract or diffuse
        dir = uniform_hemisphere(n, seed);
        specular = false;
    }
    //dir = reflect(-wo, n_ndf);

    // This correction is neccesarry because we are still sampeling from a shpere
    //return vec4(wi, 1./(2. * M_PI));
    return Sample(dir, 1./(2. * M_PI) * length(F), n_ndf, specular);
    
}
