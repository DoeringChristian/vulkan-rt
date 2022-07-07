#include "common.glsl"

float luminance(vec3 c)
{
    return 0.212671 * c.x + 0.715160 * c.y + 0.072169 * c.z;
}

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

float fresnelSchlick(float x){
    x = clamp(1.- 0, 0., 1.);
    float x2 = x*x;
    return x2 * x2 * x;
}
float fresnel_dielectric(float cosThetaI, float eta){
    float sin2ThetaT = eta * eta * (1. - cosThetaI * cosThetaI);

    if(sin2ThetaT > 1.){
        return 1.;
    }

    float cosThetaT = sqrt(max(1. - sin2ThetaT, 0.));
    
    float rs = (eta * cosThetaT - cosThetaI) / (eta * cosThetaT + cosThetaI);
    float rp = (eta * cosThetaI - cosThetaT) / (eta * cosThetaI + cosThetaT);

    return 0.5 * (rs * rs + rp * rp);
}
float fresnelDisney(float metallic, float eta, float l_dot_h, float v_dot_h){
    float Fm = fresnelSchlick(l_dot_h);
    float Fd = fresnel_dielectric(abs(v_dot_h), eta);
    return mix(Fd, Fm, metallic);
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
    if (dot(wi, hit.g) < 0.){
        wi = reflect(wi, hit.g);
    } 

    
    // Sample:
    ray.attenuation *= hit.albedo.rgb / M_PI * (2. * M_PI);
    ray.dir = wi;
}
void eval_diffuse(HitInfo hit, inout Payload ray){
    ray.attenuation *= hit.albedo.rgb / M_PI * (2. * M_PI);
}

void sample_refraction(HitInfo hit, inout Payload ray, float eta){
    vec3 m = sample_DistributionGGX(hit.roughness, hit.n, ray.seed);
    vec3 wi = refract(-hit.wo, m, eta);
    
    float wi_dot_n = max(dot(hit.n, -wi), 0.);
    float NdotV = max(dot(hit.n, wi), 0.0);
    float G = GeometrySchlickGGX(NdotV, hit.roughness);

    vec3 fr = vec3(1.);

    // Sample:
    ray.attenuation *= fr * wi_dot_n * (2. * M_PI);
    //ray.ior = n2;
    ray.dir = wi;
}
void eval_specular_refr(HitInfo hit, inout Payload ray, float eta){
    vec3 h = normalize(ray.dir + hit.wo);
    
    float wi_dot_n = max(dot(hit.n, -ray.dir), 0.);
    float NdotV = max(dot(hit.n, ray.dir), 0.);
    float G = GeometrySchlickGGX(NdotV, hit.roughness);

    ray.attenuation *= wi_dot_n * (2. * M_PI);
}

void sample_specular_refl(HitInfo hit, inout Payload ray, float eta){
    // Sample:
    vec3 m = sample_DistributionGGX(hit.roughness, hit.n, ray.seed);
    vec3 wi = reflect(-hit.wo, m);
    ray.dir = wi;
    
    if (dot(wi, hit.g) < 0.){
        //wi = reflect(wi, hit.g);
    } 

    // Eval:
    float F0 = (1. - eta) / (1. + eta);
    vec3 cspec = mix(F0 * F0 * vec3(1.), hit.albedo.rgb, hit.metallic);
    float FM = fresnelDisney(hit.metallic, eta, dot(wi, m), dot(hit.wo, m));
    vec3 F = mix(cspec, vec3(1.), FM);
    
    float wi_dot_n = max(dot(hit.n, wi), 0.);
    float G = GeometrySmith(hit.n, hit.wo, wi, hit.roughness);

    vec3 numerator = G * F;
    float denominator = 4. * max(dot(hit.n, hit.wo), 0.) * max(dot(hit.n, wi), 0.) + 0.001;
    vec3 specular = numerator/denominator;
    vec3 fr = specular;

    // Sample:
    ray.attenuation *= fr * wi_dot_n * (2 * M_PI);
    
}

void eval_specular_refl(HitInfo hit, inout Payload ray, float eta){
    vec3 wi = ray.dir;
    vec3 h = normalize(wi + hit.wo);

    if(dot(wi, hit.n) < 0.){
        ray.attenuation *= 0;
    }
    float F0 = (1. - eta) / (1. + eta);
    vec3 cspec = mix(F0 * F0 * vec3(1.), hit.albedo.rgb, hit.metallic);
    float FM = fresnelDisney(hit.metallic, eta, dot(wi, h), dot(hit.wo, h));
    vec3 F = mix(cspec, vec3(1.), FM);

    float wi_dot_n = max(dot(hit.n, wi), 0.);
    float G = GeometrySmith(hit.n, hit.wo, wi, hit.roughness);
    vec3 numerator = G * F;
    float denominator = 4. * max(dot(hit.n, hit.wo), 0.) * max(dot(hit.n, wi), 0.) + 0.001;
    vec3 specular = numerator/denominator;
    
    ray.attenuation *= specular * wi_dot_n * (2 * M_PI);
}

void sample_dielectric(HitInfo hit, inout Payload ray, float eta){
    float wo_dot_n = dot(hit.wo, hit.n);
    float F_approx = fresnelDisney(hit.metallic, eta, wo_dot_n, wo_dot_n);
    
    //float kS = fresnelSchlickReflectAmount(n1, n2, m, -hit.wo, F0);
    float kS = F_approx;
    //kS = 0.5;
    float kD = 1. - kS;

    if (randf(ray.seed) < kS){
        ray.attenuation /= kS;
        // Specular case
        sample_specular_refl(hit, ray, eta);
    }
    else{
        ray.attenuation /= kD;
        if(randf(ray.seed) >= hit.transmission){
            // Diffuse Case
            sample_diffuse(hit, ray);
        }
        else{
            // Refraction case
            sample_refraction(hit, ray, eta);
        }
    }
}

void sample_metallic(HitInfo hit, inout Payload ray, float eta){
    sample_specular_refl(hit, ray, eta);
}

//Sample generate_sample(vec3 n, vec3 wo, float dist, InterMaterial mat, float ior, vec3 seed){
void sample_shader(HitInfo hit, inout Payload ray){

    ray.orig = hit.pos;
    ray.color += ray.attenuation * hit.emission.rgb;
    hit.metallic = 0;
    hit.transmission = 0;
    hit.roughness = 1.;
    
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
    float eta = n1 / n2;
    float wo_dot_n = dot(hit.wo, hit.n);
    
    float F0 = (1. - eta) / (1. + eta);
    vec3 cspec = mix(F0 * F0 * vec3(1.), hit.albedo.rgb, hit.metallic);
    float F_approx = fresnelDisney(hit.metallic, eta, wo_dot_n, wo_dot_n);

    float p_diff = luminance(hit.albedo.rgb) * (1. - hit.metallic) * (1. - hit.transmission);
    float p_refl = luminance(mix(cspec, vec3(1.), F_approx));
    float p_refr = luminance(hit.albedo.rgb) * (1. - F_approx) * (1. - hit.metallic) * hit.transmission;
    float total = p_diff + p_refl + p_refr;
    p_diff /= total;
    p_refl /= total;
    p_refr /= total;

    float rnd = randf(ray.seed);
    if (rnd < p_diff){ // Diffuse sample/eval
        ray.attenuation /= p_diff;
        
        ray.dir = allign_hemisphere(cosine_hemisphere(ray.seed), hit.n);
        
        eval_diffuse(hit, ray);
    } else if(rnd < p_refl + p_refr + p_diff){ // Specular refl/refr
        ray.attenuation /= (p_refl + p_refr);
        vec3 m = sample_DistributionGGX(hit.roughness, hit.n, ray.seed);

        if (dot(m, hit.n) < 0){
            reflect(m, hit.n);
        }

        if (randf(ray.seed) < F_approx){ // Specular reflection
            ray.attenuation /= F_approx;
            
            vec3 wi = normalize(reflect(-hit.wo, m));
            
            eval_specular_refl(hit, ray, eta);
        } else{ // Specular refraction
            ray.attenuation /= (1. - F_approx);
            
            vec3 wi = normalize(refract(-hit.wo, m, eta));

            eval_specular_refr(hit, ray, eta);
        }
    }
}
