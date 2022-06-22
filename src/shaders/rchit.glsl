
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable

#include "rand.glsl"
#include "common.glsl"
#include "test01_brdf.glsl"


hitAttributeEXT vec2 hit_co;

layout(location = 0) rayPayloadInEXT Payload payload;

layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
layout(set = 0, binding = 1, rgba32f) uniform image2D image;
layout(std140, set = 0, binding = 2) buffer Instances{
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
    //uint model_id = inst.model;
    //uint idx0 = model_indices[id].indices[0];

    ivec3 indices = ivec3(model_indices[inst.indices].indices[3 * gl_PrimitiveID + 0],
                        model_indices[inst.indices].indices[3 * gl_PrimitiveID + 1],
                        model_indices[inst.indices].indices[3 * gl_PrimitiveID + 2]);

    vec3 barycentric = vec3(1. - hit_co.x - hit_co.y, hit_co.x, hit_co.y);

    vec3 pos0 = vec3(model_positions[inst.positions].positions[3 * indices.x + 0],
                    model_positions[inst.positions].positions[3 * indices.x + 1],
                    model_positions[inst.positions].positions[3 * indices.x + 2]);
    vec3 pos1 = vec3(model_positions[inst.positions].positions[3 * indices.y + 0],
                    model_positions[inst.positions].positions[3 * indices.y + 1],
                    model_positions[inst.positions].positions[3 * indices.y + 2]);
    vec3 pos2 = vec3(model_positions[inst.positions].positions[3 * indices.z + 0],
                    model_positions[inst.positions].positions[3 * indices.z + 1],
                    model_positions[inst.positions].positions[3 * indices.z + 2]);

    vec3 pos = pos0 * barycentric.x + pos1 * barycentric.y + pos2 * barycentric.z;
    vec3 geo_norm = normalize(cross(pos1 - pos0, pos2 - pos0));
    
    vec3 prev_pos = payload.orig;
    vec3 prev_dir = payload.dir;

    payload.orig = pos;

    vec3 wo = normalize(-prev_dir);
    vec4 wip = generate_sample(geo_norm, wo, mat, pos);
    vec3 brdf = eval(geo_norm, wo, wip.xyz, mat) / wip.w;

    payload.dir = wip.xyz;

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
