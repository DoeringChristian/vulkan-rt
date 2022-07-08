
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable

#include "rand.glsl"
#include "common.glsl"
#include "disney_bsdf01.glsl"


hitAttributeEXT vec2 hit_co;

layout(location = 0) rayPayloadInEXT Payload payload;

layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
//layout(set = 0, binding = 1, rgba32f) uniform image2D image;
layout(std140, set = 0, binding = 1) buffer Instances{
    Instance instances[];
};
layout(std140, set = 0, binding = 2) buffer Materials{
    Material materials[];
};
layout(std140, set = 0, binding = 3) buffer Indices{
    uint indices[];
}model_indices[];
layout(std140, set = 0, binding = 4) buffer Vertices{
    Vertex vertices[];
}model_vertices[];
layout(set = 0, binding = 5) uniform sampler2D textures[];

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

    ivec3 indices = ivec3(model_indices[inst.mesh_index].indices[3 * gl_PrimitiveID + 0],
                          model_indices[inst.mesh_index].indices[3 * gl_PrimitiveID + 1],
                          model_indices[inst.mesh_index].indices[3 * gl_PrimitiveID + 2]);

    vec3 barycentric = vec3(1. - hit_co.x - hit_co.y, hit_co.x, hit_co.y);

    Vertex vert0 = model_vertices[inst.mesh_index].vertices[indices.x];
    Vertex vert1 = model_vertices[inst.mesh_index].vertices[indices.y];
    Vertex vert2 = model_vertices[inst.mesh_index].vertices[indices.z];

    vec3 pos0 = vert0.pos.xyz;
    vec3 pos1 = vert1.pos.xyz;
    vec3 pos2 = vert2.pos.xyz;
    // Apply transform
    pos0 = (transform * vec4(pos0, 1.)).xyz;
    pos1 = (transform * vec4(pos1, 1.)).xyz;
    pos2 = (transform * vec4(pos2, 1.)).xyz;

    vec3 pos = pos0 * barycentric.x + pos1 * barycentric.y + pos2 * barycentric.z;
    
    //===========================================================
    // Get/Generate Normals:
    //===========================================================
    
    vec3 norm = vec3(0.);
    vec3 norm0 = vert0.normal.xyz;
    vec3 norm1 = vert1.normal.xyz;
    vec3 norm2 = vert2.normal.xyz;
    vec3 gnorm = normalize(cross(pos1 - pos0, pos2 - pos0));
    if (length(norm0) > 0.1 && length(norm1) > 0.1 && length(norm2) > 0.1){
        norm0 = normalize(norm0);
        norm1 = normalize(norm1);
        norm2 = normalize(norm2);

        norm = norm0 * barycentric.x + norm1 * barycentric.y + norm2 * barycentric.z;
        norm = normalize(inverse(transpose(mat3(transform))) * norm);
    }
    else{
        norm = gnorm;
    }

    //===========================================================
    // Interpolate Material:
    //===========================================================
    vec3 prev_pos = payload.orig;
    vec3 prev_dir = payload.dir;

    //payload.orig = pos;

    vec3 wo = normalize(-prev_dir);
    float dist = length(prev_pos - pos);
    
    HitInfo hit;
    hit.pos = pos;
    hit.wo = wo;
    hit.g = gnorm;
    hit.n = norm;
    hit.dist = dist;
    
    MatInfo matinfo;
    matinfo.albedo = mat.albedo.rgb;
    matinfo.emission = mat.emission.rgb;
    matinfo.transmission = mat.transmission;
    matinfo.metallic = mat.metallic;
    matinfo.roughness = max(mat.roughness * mat.roughness, 0.001);
    matinfo.ior = mat.ior;
    
    matinfo.anisotropic = 0.00;
    matinfo.subsurface = 0;
    matinfo.specularTint = 0;
    matinfo.sheen = 0;
    matinfo.sheenTint = 0;
    matinfo.clearcoat = 0;
    matinfo.clearcoatRoughness = 0.;
    //mat.ior = 1.4;
    matinfo.ax = 0.001;
    matinfo.ay = 0.001;

    // TODO: material interpolation and tangent space.
    vec2 uv0 = vert0.uv.xy;
    vec2 uv1 = vert1.uv.xy;
    vec2 uv2 = vert2.uv.xy;
    vec2 uv = uv0 * barycentric.x + uv1 * barycentric.y + uv2 * barycentric.z;
    if (mat.albedo_tex != INDEX_UNDEF){
        matinfo.albedo = texture(textures[mat.albedo_tex], uv).rgb;
    }
    if (mat.mr_tex != INDEX_UNDEF){
        // As specified by gltf specs the blue chanel stores metallness, the green chanel roughness.
        vec2 mr = texture(textures[mat.mr_tex], uv).bg;
        matinfo.metallic = mr.x;
        matinfo.roughness = max(mr.y * mr.y, 0.001);
    }
    if (mat.normal_tex != INDEX_UNDEF){
        mat3 TBN = compute_TBN(uv1 - uv0, uv2 - uv0, pos1 - pos0, pos2 - pos0, norm);
        
        vec3 norm_tex = texture(textures[mat.normal_tex], uv).rgb;
        // need to invert the y component of the normal texture.
        norm_tex = vec3(norm_tex.x, 1. - norm_tex.y, norm_tex.z);
        norm_tex = normalize(norm_tex * 2. - 1.);
        norm = normalize(TBN * norm_tex);
        hit.n = norm;
    }
    // DEBUG:
    //matinfo.roughness = 0.1;

    float aspect = sqrt(1. - matinfo.anisotropic * 0.9);
    matinfo.ax = max(0.001, matinfo.roughness / aspect);
    matinfo.ay = max(0.001, matinfo.roughness / aspect);

    
    //===========================================================
    // Call BRDF functions:
    //===========================================================
    
    sample_shader(hit, matinfo, payload);
    
    // thrgouhput roussian roulette propability
    //p_{RR} = max_{RGB}\leftb( \prod_{d = 1}^{D-1} \left({f_r(x_d, w_d \rightarrow v_d) cos(\theta_d)) \over p(w_d)p_{RR_d}}\right)\right)
    float p_rr = max(payload.attenuation.r, max(payload.attenuation.g, payload.attenuation.b));
    if (payload.depth < min_rr){
        p_rr = 1.;
    }
    
    payload.attenuation *= 1. / p_rr;
    
    if (randf(payload.seed) >= p_rr){
        payload.ray_active = 0;
        return;
    }
    
    payload.depth += 1;
}
