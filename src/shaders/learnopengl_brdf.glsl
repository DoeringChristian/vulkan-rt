
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


vec3 eval(vec3 n, vec3 wo, vec3 wi, Material mat){
    float roughness = mat.mra.y;
    float metallic = mat.mra.x;
    
    /*
    vec3 dir_len = prev_pos - pos;
    float distance = length(dir_len);
    float attenuation = 1. / (distance * distance);
    */
    
    vec3 albedo = mat.albedo.xyz;
    
    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);
    vec3 h = normalize(wo + wi);
    float NDF = DistributionGGX(n, h, roughness);
    float G = GeometrySmith(n, wo, wi, roughness);
    vec3 F = fresnelSchlick(max(dot(h, wo), 0.), F0);

    vec3 kS = F;
    vec3 kD = vec3(1.) - kS;
    kD *= 1. - metallic;

    vec3 numerator = NDF * G * F;
    float denominator = 4. * max(dot(n, wo), 0.) * max(dot(n, wi), 0.) + 0.0001;
    vec3 specular = numerator / denominator;

    float won = max(dot(n, wo), 0.);
    float win = max(dot(n, wi), 0.);
    
    vec3 fr = (kD * albedo / M_PI + specular);

    return fr * win;
}

// Generate a sample xyz with a probability w
vec4 generate_sample(vec3 n, vec3 seed){
    vec3 w = rand_hemisphere(n, seed);
    return vec4(w.x, w.y, w.z, 1. / (2. * M_PI ));
}
