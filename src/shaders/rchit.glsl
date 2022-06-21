
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable

#include "rand.glsl"
#include "payload.glsl"

struct Material {
    vec4 albedo;
    vec4 mra;
    vec4 emission;
};

struct Instance{
    uint mat_index;
    uint model;
};

hitAttributeEXT vec2 hit_co;

layout(location = 0) rayPayloadInEXT Payload payload;

layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
layout(set = 0, binding = 1, rgba32f) uniform image2D image;
layout(set = 0, binding = 2) buffer Instances{
    Instance instances[];
};
layout(set = 0, binding = 3) buffer Materials{
    Material materials[];
};
layout(set = 0, binding = 4) buffer Indices{
    uint indices[];
}model_indices[];
layout(set = 0, binding = 5) buffer Positions{
    float positions[];
}model_positions[];

float NDF_GGXTR(float nh, float roughness){
    float a = roughness * roughness;
    float a2 = a * a;
    float nh2 = nh * nh;
    
    float num = a2;
    float denom = (nh2 * (a2 - 1.) + 1);
    denom = M_PI * denom * denom;
    return num/denom;
}

float GSchlickGGX(float nv, float roughness){
    float r = (roughness + 1.);
    float k = (r*r)/8.;
    
    float num = nv;
    float denom = nv * (1. - k) + k;
    
    return num/denom;
}
float GSmith(float nv, float nl, float roughness){
    float ggx1 = GSchlickGGX(nv, roughness);
    float ggx2 = GSchlickGGX(nl, roughness);
    return ggx1 * ggx2;
}
vec3 FSchlick(float cosTheta, vec3 F0)
{
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}  

vec2 transfer_DGGX(vec2 uv, float roughness){
    float a = roughness * roughness;
    float a2 = a * a;
    return vec2(acos(sqrt(a2/(uv.x*(a2 - 1.) + 1.))), uv.y);
}

vec3 eval(vec3 n, vec3 wo, vec3 wi, Material mat){
    float roughness = mat.mra.y;
    float metallic = mat.mra.x;
    //vec3 dir_len = prev_pos - pos;
    //float distance = length(dir_len);
    //float attenuation = 1. / (distance * distance);
    vec3 albedo = mat.albedo.xyz;
    vec3 h = normalize(wo + wi);
    float nl = max(dot(n, wi), 0.);
    float nv = max(dot(n, wo), 0.);
    float nh = max(dot(n, h), 0.);
    float hv = max(dot(h, wo), 0.);

    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo, metallic);

    
    float NDF = NDF_GGXTR(nh, roughness);
    float G = GSmith(nv, nl, roughness);
    vec3 F = FSchlick(hv, F0);

    vec3 kS = F;
    vec3 kD = vec3(1.) - kS;
    kD *= 1. - metallic;

    vec3 numerator = NDF * G * F;
    float denom = 4. * nv * nl + 0.0001;
    vec3 specular = numerator / denom;

    vec3 fr = (kD * albedo / M_PI + specular);
    //vec3 fr = albedo.xyz;

    return fr * nl;
}

float pdf(vec3 w){
    return 1. / (2. * M_PI);
}

vec3 generate_sample(vec3 n, vec3 seed){
    return rand_hemisphere(n, seed);
}

void main() {
    if (payload.ray_active == 0) {
        return;
    }
    /*
    if (payload.depth >= 2){
        return;
    }
*/

    const uint min_rr = 2;

    //===========================================================
    // Extract geometry information:
    //===========================================================
    Instance inst = instances[gl_InstanceCustomIndexEXT];
    Material mat = materials[inst.mat_index];
    uint model_id = inst.model;
    //uint idx0 = model_indices[id].indices[0];

    ivec3 indices = ivec3(model_indices[model_id].indices[3 * gl_PrimitiveID + 0],
                        model_indices[model_id].indices[3 * gl_PrimitiveID + 1],
                        model_indices[model_id].indices[3 * gl_PrimitiveID + 2]);

    vec3 barycentric = vec3(1. - hit_co.x - hit_co.y, hit_co.x, hit_co.y);

    vec3 pos0 = vec3(model_positions[model_id].positions[3 * indices.x + 0],
                    model_positions[model_id].positions[3 * indices.x + 1],
                    model_positions[model_id].positions[3 * indices.x + 2]);
    vec3 pos1 = vec3(model_positions[model_id].positions[3 * indices.y + 0],
                    model_positions[model_id].positions[3 * indices.y + 1],
                    model_positions[model_id].positions[3 * indices.y + 2]);
    vec3 pos2 = vec3(model_positions[model_id].positions[3 * indices.z + 0],
                    model_positions[model_id].positions[3 * indices.z + 1],
                    model_positions[model_id].positions[3 * indices.z + 2]);

    vec3 pos = pos0 * barycentric.x + pos1 * barycentric.y + pos2 * barycentric.z;
    vec3 geo_norm = normalize(cross(pos1 - pos0, pos2 - pos0));
    
    vec3 prev_pos = payload.orig;
    vec3 prev_dir = payload.dir;

    payload.orig = pos;

    vec3 w = generate_sample(geo_norm, pos);
    vec3 brdf = eval(geo_norm, normalize(- prev_dir), w, mat) / pdf(w);

    payload.dir = w;

    // thrgouhput roussian roulette propability
    //p_{RR} = max_{RGB}\leftb( \prod_{d = 1}^{D-1} \left({f_r(x_d, w_d \rightarrow v_d) cos(\theta_d)) \over p(w_d)p_{RR_d}}\right)\right)
    float p_rr = max(payload.attenuation.r, max(payload.attenuation.g, payload.attenuation.b));
    if (payload.depth < min_rr){
        p_rr = 1.;
    }

    payload.color += payload.attenuation * mat.emission.xyz;
    payload.attenuation *= brdf / p_rr;

    //payload.prop *= p_rr;
    
    if (rand(vec3(payload.dir)) >= p_rr){
        payload.ray_active = 0;
        return;
    }

    payload.depth += 1;
}
