#ifndef UTILS_GLSL
#define UTILS_GLSL

#include "bindings.glsl"

mat3 compute_TBN(vec2 duv0, vec2 duv1, vec3 dpos0, vec3 dpos1, vec3 n){
    float r = 1./(duv0.x * duv1.y - duv0.y * duv1.x);
    vec3 t = (dpos0 * duv1.y - dpos1 * duv0.y)*r;
    vec3 b = (dpos1 * duv0.x - dpos0 * duv1.x)*r;
    return mat3(t, b, n);
}

void hitInfo(
    in uint instanceIndex,
    in uint primitiveID,
    in vec2 hit_co,
    out HitInfo hit,
    out Material mat){
    //===========================================================
    // Extract geometry information:
    //===========================================================
    InstanceData inst = instances[instanceIndex];
    mat4 transform = mat4(inst.trans0, inst.trans1, inst.trans2, inst.trans3);
    MaterialData materialData = materials[inst.mat_index];

    Indices indices = Indices(inst.indices);
    Vertices vertices = Vertices(inst.vertices);

    ivec3 tri = ivec3(indices.i[3 * primitiveID + 0],
                      indices.i[3 * primitiveID + 1],
                      indices.i[3 * primitiveID + 2]);

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
    //vec3 prev_pos = payload.orig;
    //vec3 prev_dir = payload.dir;

    //payload.orig = pos;

    //vec3 wo = normalize(-prev_dir);
    //float dist = length(prev_pos - pos);
    
    // Initialize hit
    //HitInfo hit;
    hit.pos = pos;
    //hit.wo = wo;
    hit.g = gnorm;
    hit.n = norm;
    //hit.dist = dist;
    
    // Initialize Material
    //Material mat;
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
    mat.med.anisotropic = clamp(materialData.med.anisotropic, -0.9, 0.9);
    mat.med.density = materialData.med.density;

    //===========================================================
    // Get Textures of Material:
    //===========================================================
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
        // TODO: maybe mat.roughness should not be a but roughness
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

    /*
    if (dot(hit.g, hit.wo) < 0.){
        med = mat.med;
    }else{
        med = payload.med;
    }
*/
}

#endif //UTILS_GLSL
