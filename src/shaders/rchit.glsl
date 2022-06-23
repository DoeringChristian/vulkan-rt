
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
// 6 bindings per set seem to be max.
layout(set = 0, binding = 6) buffer Normals{
    float normals[];
}model_normals[];
layout(set = 1, binding = 0) buffer TexCoords{
    float tex_coords[];
}model_tex_coords[];
layout(set = 1, binding = 1) uniform sampler2D textures[];

mat3 compute_TBN(vec2 duv0, vec2 duv1, vec3 dpos0, vec3 dpos1, vec3 n){
    float r = 1./(duv0.x * duv1.y - duv0.y * duv1.x);
    vec3 t = (dpos0 * duv1.y - dpos1 * duv0.y)*r;
    vec3 b = (dpos1 * duv0.x - dpos0 * duv1.x)*r;
    return mat3(t, b, n);
}

void main() {
    if (payload.ray_active == 0) {
        return;
    }

    const uint min_rr = 2;

    //===========================================================
    // Extract geometry information:
    //===========================================================
    Instance inst = instances[gl_InstanceCustomIndexEXT];
    mat4 transform = mat4(inst.trans0, inst.trans1, inst.trans2, inst.trans3);
    Material mat = materials[inst.mat_index];

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
    // Apply transform
    pos0 = (transform * vec4(pos0, 1.)).xyz;
    pos1 = (transform * vec4(pos1, 1.)).xyz;
    pos2 = (transform * vec4(pos2, 1.)).xyz;

    vec3 pos = pos0 * barycentric.x + pos1 * barycentric.y + pos2 * barycentric.z;
    
    // get or generate normals
    
    vec3 norm = vec3(0.);
    vec3 norm0 = vec3(0.);
    vec3 norm1 = vec3(0.);
    vec3 norm2 = vec3(0.);
    if (inst.normals != INDEX_UNDEF){
        norm0 = vec3(model_normals[inst.normals].normals[3 * indices.x + 0],
                     model_normals[inst.normals].normals[3 * indices.x + 1],
                     model_normals[inst.normals].normals[3 * indices.x + 2]);
        norm1 = vec3(model_normals[inst.normals].normals[3 * indices.y + 0],
                     model_normals[inst.normals].normals[3 * indices.y + 1],
                     model_normals[inst.normals].normals[3 * indices.y + 2]);
        norm2 = vec3(model_normals[inst.normals].normals[3 * indices.z + 0],
                     model_normals[inst.normals].normals[3 * indices.z + 1],
                     model_normals[inst.normals].normals[3 * indices.z + 2]);
        norm0 = normalize(norm0);
        norm1 = normalize(norm1);
        norm2 = normalize(norm2);

        norm = norm0 * barycentric.x + norm1 * barycentric.y + norm2 * barycentric.z;
        norm = normalize(inverse(transpose(mat3(transform))) * norm);
    }
    else{
        norm = normalize(cross(pos1 - pos0, pos2 - pos0));
    }

    // Interpolate materials
    InterMaterial inter_mat = InterMaterial(mat.albedo, vec2(mat.mr.x, mat.mr.y), mat.emission);

    // TODO: material interpolation and tangent space.
    if (mat.albedo_tex != INDEX_UNDEF && mat.albedo_texco != INDEX_UNDEF){
        vec2 texco0 = vec2(model_tex_coords[inst.tex_coords + mat.albedo_texco].tex_coords[2 * indices.x + 0],
                           model_tex_coords[inst.tex_coords + mat.albedo_texco].tex_coords[2 * indices.x + 1]);
        vec2 texco1 = vec2(model_tex_coords[inst.tex_coords + mat.albedo_texco].tex_coords[2 * indices.y + 0],
                           model_tex_coords[inst.tex_coords + mat.albedo_texco].tex_coords[2 * indices.y + 1]);
        vec2 texco2 = vec2(model_tex_coords[inst.tex_coords + mat.albedo_texco].tex_coords[2 * indices.z + 0],
                           model_tex_coords[inst.tex_coords + mat.albedo_texco].tex_coords[2 * indices.z + 1]);
        vec2 texco = texco0 * barycentric.x + texco1 * barycentric.y + texco2 * barycentric.z;
        inter_mat.albedo = texture(textures[mat.albedo_tex], texco);
    }
    if (mat.mr_tex != INDEX_UNDEF && mat.mr_tex != INDEX_UNDEF){
        vec2 texco0 = vec2(model_tex_coords[inst.tex_coords + mat.mr_texco].tex_coords[2 * indices.x + 0],
                           model_tex_coords[inst.tex_coords + mat.mr_texco].tex_coords[2 * indices.x + 1]);
        vec2 texco1 = vec2(model_tex_coords[inst.tex_coords + mat.mr_texco].tex_coords[2 * indices.y + 0],
                           model_tex_coords[inst.tex_coords + mat.mr_texco].tex_coords[2 * indices.y + 1]);
        vec2 texco2 = vec2(model_tex_coords[inst.tex_coords + mat.mr_texco].tex_coords[2 * indices.z + 0],
                           model_tex_coords[inst.tex_coords + mat.mr_texco].tex_coords[2 * indices.z + 1]);
        vec2 texco = texco0 * barycentric.x + texco1 * barycentric.y + texco2 * barycentric.z;
        // As specified by gltf specs the blue chanel stores metallness, the green chanel roughness.
        inter_mat.mr = texture(textures[mat.mr_tex], texco).bg;
    }
    if (mat.normal_tex != INDEX_UNDEF && mat.normal_texco != INDEX_UNDEF){
        vec2 texco0 = vec2(model_tex_coords[inst.tex_coords + mat.normal_texco].tex_coords[2 * indices.x + 0],
                           model_tex_coords[inst.tex_coords + mat.normal_texco].tex_coords[2 * indices.x + 1]);
        vec2 texco1 = vec2(model_tex_coords[inst.tex_coords + mat.normal_texco].tex_coords[2 * indices.y + 0],
                           model_tex_coords[inst.tex_coords + mat.normal_texco].tex_coords[2 * indices.y + 1]);
        vec2 texco2 = vec2(model_tex_coords[inst.tex_coords + mat.normal_texco].tex_coords[2 * indices.z + 0],
                           model_tex_coords[inst.tex_coords + mat.normal_texco].tex_coords[2 * indices.z + 1]);
        vec2 texco = texco0 * barycentric.x + texco1 * barycentric.y + texco2 * barycentric.z;
        
        mat3 TBN = compute_TBN(texco1 - texco0, texco2 - texco0, pos1 - pos0, pos2 - pos0, norm);
        
        vec3 norm_tex = texture(textures[mat.normal_tex], texco).rgb;
        norm_tex = normalize(norm_tex * 2. - 1.);
        norm = normalize(TBN * norm_tex);
    }
    
    vec3 prev_pos = payload.orig;
    vec3 prev_dir = payload.dir;

    payload.orig = pos;

    vec3 wo = normalize(-prev_dir);
    vec4 wip = generate_sample(norm, wo, inter_mat, pos);
    vec3 brdf = eval(norm, wo, wip.xyz, inter_mat) / wip.w;

    payload.dir = wip.xyz;

    // thrgouhput roussian roulette propability
    //p_{RR} = max_{RGB}\leftb( \prod_{d = 1}^{D-1} \left({f_r(x_d, w_d \rightarrow v_d) cos(\theta_d)) \over p(w_d)p_{RR_d}}\right)\right)
    float p_rr = max(payload.attenuation.r, max(payload.attenuation.g, payload.attenuation.b));
    if (payload.depth < min_rr){
        p_rr = 1.;
    }

    payload.color += payload.attenuation * inter_mat.emission.xyz;
    payload.attenuation *= brdf / p_rr;

    // DEBUG:
    //payload.color = vec3(inter_mat.mr.y);
    
    if (rand(vec3(payload.dir)) >= p_rr){
        payload.ray_active = 0;
        return;
    }

    payload.depth += 1;
}
