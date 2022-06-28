
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
/*
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
*/
float fresnelSchlickReflectAmount(float n1, float n2, vec3 normal, vec3 incident, float f0){
    float r0 = (n1-n2)/(n1+n2);
    r0 *= r0;
    float cosTheta = -dot(normal, incident);
    if (n1 > n2)
    {
        float n = n1/n2;
        float sin2Theta = n*n*(1.0- cosTheta * cosTheta);
        // Total internal reflection
        if (sin2Theta > 1.0)
            return 1.;
        cosTheta = sqrt(1.0 - sin2Theta);
    }
    float x = 1.0 - cosTheta;
    float ret = r0 + (1. - r0) * x * x * x * x * x;
    return mix(f0, 1., ret);
}

// from https://agraphicsguy.wordpress.com/2015/11/01/sampling-microfacet-brdf/
vec3 sample_DistributionGGX(float roughness, vec3 n, vec3 seed){
    float a = roughness * roughness;
    float a2 = a * a;

    vec2 e = rand2(seed);

    float theta = acos(sqrt((1. - e.x)/((a2 - 1.) * e.x + 1.)));
    float phi = 2. * M_PI * e.y;

    vec3 m = vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );

    return allign_hemisphere(m, n);
}

vec3 sample_DistributionBeckmann(float roughness, vec3 n, vec3 seed){
    float a = roughness * roughness;
    float a2 = a * a;

    vec2 e = rand2(seed);

    float theta = acos(sqrt(1./(1. - a2 * log(1. - e.x))));
    float phi = 2. * M_PI * e.y;

    vec3 m = vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );

    return allign_hemisphere(m, n);
}

//Sample generate_sample(vec3 n, vec3 wo, float dist, InterMaterial mat, float ior, vec3 seed){
void sample_shader(HitInfo hit, inout Payload ray, vec3 seed){

    ray.orig = hit.pos;
    ray.color += ray.attenuation * hit.emission.rgb * 2.;
    //ray.color = hit.n;
    
    float metallic = hit.metallic;
    float roughness = hit.roughness;
    vec3 albedo = hit.albedo.rgb;

    
    // Accumulative ior should work since n3/n2 = (n3 * n2 * n1) / (n2 * n1);
    float n1 = 1.;
    float n2 = 1.;
    if (dot(hit.n, hit.wo) < 0){
        // From inside of material
        hit.n = -hit.n;
        n2 = hit.ior;
        n1 = 1.;
    }else{
        // From outside of material
        n2 = 1.;
        n1 = hit.ior;
    }
    
    // m is the microfacet normal
    //vec3 m = sample_DistributionGGX(roughness, hit.n, seed - vec3(M_PI));
    vec3 m = sample_DistributionBeckmann(roughness, hit.n, seed - vec3(M_PI));
    
    float F0_sqrt = (n1 - n2) / (n1 + n2);
    vec3 F0 = vec3(F0_sqrt * F0_sqrt);
    F0 = mix(F0, albedo, metallic);
    float F0_avg = (F0.x+F0.y+F0.z)/3.;

    vec3 F = fresnelSchlick(dot(m, hit.wo), F0);
    //F = vec3(0.);

    
    //float kS = fresnelSchlickReflectAmount(max(0, dot(m, hit.wo)), n1, n2, F0_avg);
    float kS = fresnelSchlickReflectAmount(n1, n2, m, -hit.wo, F0_avg);
    float kD = 1. - kS;

    if (rand(seed + vec3(M_PI)) < kS){
        // Specular case

        vec3 wi = reflect(-hit.wo, m);
        float wi_dot_n = max(dot(m, wi), 0.);
        float G = GeometrySmith(hit.n, hit.wo, wi, roughness);

        vec3 numerator = G * vec3(1.);
        float denominator = 4. * max(dot(m, hit.wo), 0.) * max(dot(m, wi), 0.) + 0.001;
        vec3 specular = numerator/denominator;
        vec3 fr = specular * F;

        // Sample:
        ray.attenuation *= fr * wi_dot_n * (2 * M_PI) / kS;
        ray.dir = wi;
    }
    else{

        if(rand(seed - vec3(M_PI * M_PI)) >= hit.transmission){
            // Diffuse Case
            vec3 wi = allign_hemisphere(uniform_hemisphere(seed), hit.n);
            float wi_dot_n = max(dot(hit.n, wi), 0.);

            vec3 fr =  (1. - metallic) * albedo / M_PI;

            // Sample:
            ray.attenuation *= fr * wi_dot_n * (2. * M_PI);
            ray.dir = wi;
        }
        else{
            // Refraction case
            vec3 wi = refract(-hit.wo, hit.n, n2/n1);
            float wi_dot_n = max(dot(m, wi), 0.);

            vec3 fr = vec3(1.);

            // Sample:
            ray.attenuation *= fr * (2. * M_PI) * (1. - F) / kD;
            //ray.ior = n2;
            ray.dir = wi;
        }
    }
    // Debug:
    //ray.color = ray.dir;
    //ray.color = vec3(kS);
}
