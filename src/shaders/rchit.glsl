
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "rand.glsl"
#include "common.glsl"
#include "disney_bsdf01.glsl"


hitAttributeEXT vec2 hit_co;

layout(location = 0) rayPayloadInEXT Payload payload;

layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
//layout(set = 0, binding = 1, rgba32f) uniform image2D image;
layout(std140, set = 0, binding = 1) buffer Instances{
    InstanceData instances[];
};
layout(std140, set = 0, binding = 2) buffer Materials{
    MaterialData materials[];
};
layout(set = 0, binding = 3) uniform sampler2D textures[];
layout(set = 0, binding = 4) buffer Lights{
    uvec4 count;
    LightData l[];
}lights;

layout(buffer_reference, scalar) buffer Indices{
    uint i[];
};
layout(buffer_reference, scalar) buffer Vertices{
    Vertex v[];
};

mat3 compute_TBN(vec2 duv0, vec2 duv1, vec3 dpos0, vec3 dpos1, vec3 n){
    float r = 1./(duv0.x * duv1.y - duv0.y * duv1.x);
    vec3 t = (dpos0 * duv1.y - dpos1 * duv0.y)*r;
    vec3 b = (dpos1 * duv0.x - dpos0 * duv1.x)*r;
    return mat3(t, b, n);
}

void main() {
    init_seed(payload.seed);
    if (payload.ray_active == 0) {
        return;
    }

    const uint min_rr = 2;

    //===========================================================
    // Extract geometry information:
    //===========================================================
    InstanceData inst = instances[gl_InstanceCustomIndexEXT];
    mat4 transform = mat4(inst.trans0, inst.trans1, inst.trans2, inst.trans3);
    MaterialData materialData = materials[inst.mat_index];

    Indices indices = Indices(inst.indices);
    Vertices vertices = Vertices(inst.vertices);

    ivec3 tri = ivec3(indices.i[3 * gl_PrimitiveID + 0],
                      indices.i[3 * gl_PrimitiveID + 1],
                      indices.i[3 * gl_PrimitiveID + 2]);

    vec3 barycentric = vec3(1. - hit_co.x - hit_co.y, hit_co.x, hit_co.y);

    Vertex vert0 = vertices.v[tri.x];
    Vertex vert1 = vertices.v[tri.y];
    Vertex vert2 = vertices.v[tri.z];

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
    
    // Initialize hit
    HitInfo hit;
    hit.pos = pos;
    hit.wo = wo;
    hit.g = gnorm;
    hit.n = norm;
    hit.dist = dist;
    
    // Initialize Material
    Material mat;
    mat.albedo = materialData.albedo.rgb;
    mat.emission = materialData.emission.rgb;
    mat.transmission = materialData.transmission;
    mat.metallic = materialData.metallic;
    mat.roughness = max(materialData.roughness * materialData.roughness, 0.001);
    mat.ior = materialData.ior;
    
    mat.anisotropic = 0.00;
    mat.subsurface = 0;
    mat.specularTint = 0;
    mat.sheen = 0;
    mat.sheenTint = 0;
    mat.clearcoat = 0;
    mat.clearcoatRoughness = 0.;
    //mat.ior = 1.4;
    mat.ax = 0.001;
    mat.ay = 0.001;

    // Initialize medium of the material the ray hits.
    mat.med.color = materialData.med.color.rgb;
    mat.med.anisotropic = materialData.med.anisotropic;
    mat.med.density = materialData.med.density;

    // TODO: material interpolation and tangent space.
    vec2 uv0 = vert0.uv.xy;
    vec2 uv1 = vert1.uv.xy;
    vec2 uv2 = vert2.uv.xy;
    vec2 uv = uv0 * barycentric.x + uv1 * barycentric.y + uv2 * barycentric.z;
    if (materialData.albedo_tex != INDEX_UNDEF){
        mat.albedo = texture(textures[materialData.albedo_tex], uv).rgb;
    }
    if (materialData.mr_tex != INDEX_UNDEF){
        // As specified by gltf specs the blue chanel stores metallness, the green chanel roughness.
        vec2 mr = texture(textures[materialData.mr_tex], uv).bg;
        mat.metallic = mr.x;
        mat.roughness = max(mr.y * mr.y, 0.001);
    }
    if (materialData.normal_tex != INDEX_UNDEF){
        mat3 TBN = compute_TBN(uv1 - uv0, uv2 - uv0, pos1 - pos0, pos2 - pos0, norm);
        
        vec3 norm_tex = texture(textures[materialData.normal_tex], uv).rgb;
        // need to invert the y component of the normal texture.
        norm_tex = vec3(norm_tex.x, 1. - norm_tex.y, norm_tex.z);
        norm_tex = normalize(norm_tex * 2. - 1.);
        norm = normalize(TBN * norm_tex);
        hit.n = norm;
    }
    // DEBUG:
    //material.roughness = 0.1;

    float aspect = sqrt(1. - mat.anisotropic * 0.9);
    mat.ax = max(0.001, mat.roughness / aspect);
    mat.ay = max(0.001, mat.roughness / aspect);

    //===========================================================
    // Call BRDF functions:
    //===========================================================

    // Sample light
    uint lightIndex = randu(lights.count.x);
    
    vec3 radiance;
    float pdf;
    vec3 f;
    sample_shader(
            hit, 
            mat, 
            payload.med, 
            payload.orig, 
            payload.dir, 
            radiance, 
            f, 
            pdf);
        

    payload.radiance += radiance * payload.throughput;
    
    if(pdf != 0.){
        payload.throughput *= f/pdf;
    }
    
    // thrgouhput roussian roulette propability
    //p_{RR} = max_{RGB}\leftb( \prod_{d = 1}^{D-1} \left({f_r(x_d, w_d \rightarrow v_d) cos(\theta_d)) \over p(w_d)p_{RR_d}}\right)\right)
    float p_rr = max(payload.throughput.r, max(payload.throughput.g, payload.throughput.b));
    if (payload.depth < min_rr){
        p_rr = 1.;
    }
    
    payload.throughput *= 1. / p_rr;
    
    if (randf(payload.seed) >= p_rr){
        payload.ray_active = 0;
        return;
    }
    
    payload.depth += 1;
}
