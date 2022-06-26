
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

struct Sample{
    vec3 dir;
    float prop;
};

Evaluation eval(vec3 n, vec3 wo, Sample s, InterMaterial mat){
    float roughness = mat.mr.y;
    float metallic = mat.mr.x;
    
    vec3 albedo = mat.albedo.xyz;
    
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    // The halfway vector is the normal sampled by the NDF
    vec3 n_ndf = normalize(wo + s.dir);
    //float NDF = DistributionGGX(n, h, roughness);
    float G = GeometrySmith(n, wo, s.dir, roughness);
    vec3 F = fresnelSchlick(max(dot(n_ndf, wo), 0.), F0);

    vec3 kS = F;
    vec3 kD = vec3(1.) - kS;
    kD *= 1. - metallic;

    vec3 numerator = G * F;
    float denominator = 4. * max(dot(n, wo), 0.) * max(dot(n, s.dir), 0.) + 0.0001;
    vec3 specular = numerator / denominator;

    float won = max(dot(n, wo), 0.);
    float win = max(dot(n, s.dir), 0.);
    
    vec3 fr = (kD * albedo / M_PI + specular);

    return Evaluation(fr * win / s.prop, s.dir);
}

// from https://agraphicsguy.wordpress.com/2015/11/01/sampling-microfacet-brdf/
// Generate a sample xyz with a probability w
Sample generate_sample(vec3 n, vec3 wo, InterMaterial mat, vec3 seed){
    float roughness = mat.mr.y;
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

    /*
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, mat.albedo.xyz, mat.mr.x);
    */

    //vec3 wi = 2. * dot(wo, h) * h - wo;
    vec3 wi = reflect(-wo, n_ndf);

    // This correction is neccesarry because we are still sampeling from a shpere
    //return vec4(wi, 1./(2. * M_PI));
    return Sample(wi, 1./(2. * M_PI));
    
}
