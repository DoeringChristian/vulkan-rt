#include "common.glsl"

// From LearnOpenGL
float eval_DistributionGGX(vec3 N, vec3 H, float roughness)
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

// from https://agraphicsguy.wordpress.com/2015/11/01/sampling-microfacet-brdf/
vec3 sample_DistributionGGX(float roughness, vec3 n, inout uint seed){
    float a = roughness * roughness;
    float a2 = a * a;

    vec2 e = rand2f(seed);

    float theta = acos(sqrt((1. - e.x)/((a2 - 1.) * e.x + 1.)));
    float phi = 2. * M_PI * e.y;

    vec3 m = vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );

    return allign_hemisphere(m, n);
}

float pdf_DistirbutionGGX(vec3 n, vec3 m, float roughness){
    float a = roughness * roughness;
    float a2 = a*a;
    float cos_t = max(dot(n, m), 0.);
    float cos2_t = cos_t * cos_t;
    float sin_t = sqrt(1. - cos2_t);

    float num = a2 * cos_t * sin_t;
    float denom = (cos2_t * (a2 - 1.) + 1.);
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
float fresnel_schlick(float cosTheta){
    float x = clamp(1. - cosTheta, 0., 1.);
    float x2 = x*x;
    return x2 * x2 * x;
}
float fresnel_dielectric(float cosThetaI, float eta){
    float sin2ThetaT = eta * eta * (1. - cosThetaI * cosThetaI);

    if (sin2ThetaT > 1.){
        return 1.;
    }
    float cosThetaT = sqrt(max(1. - sin2ThetaT, 0.));
    float rs = (eta * cosThetaT - cosThetaI) / (eta * cosThetaT - cosThetaI);
    float rp = (eta * cosThetaI - cosThetaT) / (eta * cosThetaI - cosThetaT);
    return 0.5 * (rs * rs + rp * rp);
}
vec3 fresnelSchlick(float cosTheta, vec3 R0)
{
    return R0 + (1.0 - R0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}
float fresnelSchlick(float cosTheta, float R0){
    float x = 1. - cosTheta;
    return R0 + (1. - R0) * x * x * x * x * x;
}
float fresnelSchlick(float cosTheta, float n1, float n2){
    float r0 = (n1-n2)/(n1+n2);
    r0 *= r0;
    if (n1 > n2){
        float n = n1/n2;
        float sin2Theta = n*n*(1. - cosTheta * cosTheta);
        if (sin2Theta > 1.){
            return 1.;
        }
        cosTheta = sqrt(1. - sin2Theta);
    }
    return fresnelSchlick(cosTheta, r0);
}
float fresnelSchlick(float cosTheta, float n1, float n2, float f0, float f90){
    float r0 = (n1-n2)/(n1+n2);
    r0 *= r0;
    if (n1 > n2){
        float n = n1/n2;
        float sin2Theta = n*n*(1. - cosTheta * cosTheta);
        if (sin2Theta > 1.){
            return f90;
        }
        cosTheta = sqrt(1. - sin2Theta);
    }
    
    float x = 1.0 - cosTheta;
    float ret = r0 + (1. - r0) * x * x * x * x * x;
    return mix(f0, f90, ret);
}
//https://blog.demofox.org/2020/06/14/casual-shadertoy-path-tracing-3-fresnel-rough-refraction-absorption-orbit-camera/
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
vec3 sample_DistributionBeckmann(float roughness, vec3 n, inout uint seed){
    float a = roughness * roughness;
    float a2 = a * a;

    vec2 e = rand2f(seed);

    float theta = acos(sqrt(1./(1. - a2 * log(1. - e.x))));
    float phi = 2. * M_PI * e.y;

    vec3 m = vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );

    return normalize(allign_hemisphere(m, n));
}

void sample_diffuse(HitInfo hit, inout Payload ray){
    // cosine sample along the geometry normal to prevent samples insid the geometry
    // TODO: not best approach leads to fireflies
    vec3 wi = allign_hemisphere(cosine_hemisphere(ray.seed), hit.n);
    
    // Sample:
    ray.dir = wi;
}

void eval_diffuse(HitInfo hit, inout Payload ray){
    ray.attenuation *= hit.albedo.rgb / M_PI * (2. * M_PI);
}

void sample_specular_refr(HitInfo hit, inout Payload ray, float n1, float n2){
    vec3 m = sample_DistributionGGX(hit.roughness, hit.n, ray.seed);
    vec3 wi = refract(-hit.wo, m, n1/n2);

    if (dot(wi, hit.n) < 0){
        reflect(wi, hit.n);
    }

    // Sample:
    ray.dir = wi;
}

void eval_specular_refr(HitInfo hit, inout Payload ray, float n1, float n2){
    float wi_dot_n = max(dot(hit.n, ray.dir), 0.);
    float G = GeometrySchlickGGX(wi_dot_n, hit.roughness);

    ray.attenuation *= wi_dot_n * (2. * M_PI);
}

void sample_specular_refl(HitInfo hit, inout Payload ray){
    vec3 m = sample_DistributionGGX(hit.roughness, hit.n, ray.seed);
    vec3 wi = reflect(-hit.wo, m);

    // Sample:
    ray.dir = wi;
}

void eval_specular_refl(HitInfo hit, inout Payload ray){
    float wi_dot_n = max(dot(hit.n, ray.dir), 0.);
    float G = GeometrySmith(hit.n, hit.wo, ray.dir, hit.roughness);

    vec3 numerator = G * vec3(1.);
    float denominator = 4. * max(dot(hit.n, hit.wo), 0.) * max(dot(hit.n, ray.dir), 0.) + 0.001;
    vec3 specular = numerator/denominator;

    ray.attenuation *= specular * wi_dot_n * (2. * M_PI);
}

float luminance(vec3 c){
    return 0.212671 * c.r + 0.715160 * c.g + 0.072169 * c.b;
}


void sample_bsdf(HitInfo hit, inout Payload ray, float n1, float n2){

    float wo_dot_n = dot(hit.n, hit.wo);
    float f_m = fresnel_schlick(wo_dot_n);
    float f_d = fresnel_dielectric(abs(wo_dot_n), n1/n2);
    float F = mix(f_d, f_m, hit.metallic);

    vec3 spec_color = mix(vec3(0.04), hit.albedo.rgb, hit.metallic);
    
    float p_diff = luminance(hit.albedo.rgb) * (1. - hit.metallic);
    float p_refl = luminance(mix(spec_color, vec3(1.), F));
    float p_refr = (1. - F) * (1. - hit.metallic) * hit.transmission * luminance(hit.albedo.rgb);
    float total = p_diff + p_refl + p_refr;
    p_diff /= total;
    p_refl /= total;
    p_refr /= total;

    float rnd = randf(ray.seed);

    if (rnd < p_diff){
        // Diffuse
        ray.attenuation /= p_diff;
        sample_diffuse(hit, ray);
        eval_diffuse(hit, ray);
        
    } else if (rnd < p_refl + p_diff){
        // Reflection
        ray.attenuation /= p_refl;
        sample_specular_refl(hit, ray);
        eval_specular_refl(hit, ray);
        
    } else if (rnd < p_refr + p_diff){
        // Refraction
        ray.attenuation /= p_refr;
        sample_specular_refr(hit, ray, n1, n2);
        eval_specular_refr(hit, ray, n1, n2);
        
    }
}

void eval_bsdf(HitInfo hit, inout Payload ray, float n1, float n2){
    // Reflectance case

    if (dot(hit.n, ray.dir) < 0){
        ray.attenuation *= 0;
        return;
    }
    float wo_dot_n = max(dot(hit.wo, hit.n), 0.);
    vec3 F0 = vec3((n1 - n2)/(n1 + n2));
    F0 = mix(F0, hit.albedo.rgb, hit.metallic);

    vec3 h = normalize(hit.wo + ray.dir);
    float D = eval_DistributionGGX(hit.n, h, hit.roughness);
    float G = GeometrySmith(hit.n, hit.wo, ray.dir, hit.roughness);
    vec3 F = fresnelSchlick(max(dot(h, ray.dir), 0.), F0);

    vec3 kS = F;
    vec3 kD = vec3(1.) - kS;
    kD *= 1. - hit.metallic;

    float numerator = D * G;
    float denominator = 4. * max(dot(hit.n, hit.wo), 0.) * max(dot(hit.n, ray.dir), 0.) + 0.001;
    float specular = numerator / denominator;

    ray.attenuation *= (kD * hit.albedo.rgb / M_PI + kS * specular) * wo_dot_n;
    //ray.attenuation *= hit.albedo.rgb / M_PI;
}

//Sample generate_sample(vec3 n, vec3 wo, float dist, InterMaterial mat, float ior, vec3 seed){
void sample_shader(HitInfo hit, inout Payload ray){

    ray.orig = hit.pos;
    ray.color += ray.attenuation * hit.emission.rgb;
    //hit.transmission = 0;
    //hit.roughness = 1;
    
    // DEBUG:
    
    float n1 = 1.;
    float n2 = 1.;
    if (dot(hit.g, hit.wo) < 0.){
        // From inside of material
        // Flip normals
        hit.n = -hit.n;
        hit.g = -hit.g;
        
        // set ior
        n1 = hit.ior;
        n2 = 1.;
        // Attenuate for absorbtion of transparrent materials
        ray.attenuation *= exp(-2 * M_PI / hit.albedo.rgb * hit.dist);
    }else{
        // From outside of material
        n1 = 1.;
        n2 = hit.ior;
    }

    sample_bsdf(hit, ray, n1, n2);
    //eval_bsdf(hit, ray, n1, n2);

    //ray.color = ray.attenuation -1;
    
}
