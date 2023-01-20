#ifndef UTILS_GLSL
#define UTILS_GLSL

#include "bindings.glsl"

vec3 to_local(in SurfaceInteraction si, vec3 v){
    return inverse(si.tbn) * v;
}
vec3 to_world(in SurfaceInteraction si, vec3 v){
    return si.tbn * v;
}

// Return cosinus theta in surface alligned coordinate frame
float cos_theta(vec3 v){
    return v.z;
}

float mis_weight(float pdf_a, float pdf_b){
    float a2 = pdf_a * pdf_a;
    if (pdf_a > 0){
        return a2 / (pdf_a * pdf_b + a2);
    }else{
        return 0;
    }
}

mat3 compute_TBN(vec2 duv0, vec2 duv1, vec3 dpos0, vec3 dpos1, vec3 n){
    float r = 1./(duv0.x * duv1.y - duv0.y * duv1.x);
    vec3 t = (dpos0 * duv1.y - dpos1 * duv0.y)*r;
    vec3 b = (dpos1 * duv0.x - dpos0 * duv1.x)*r;
    return mat3(t, b, n);
}

vec3 eval_texture(in Texture tex, in SurfaceInteraction si){
    if (tex.ty == TEXTURE_TY_CONSTANT){
        return tex.val;
    }else if(tex.ty == TEXTURE_TY_IMAGE){
        return texture(textures[tex.texture], si.uv).rgb;
    }
    return vec3(0.);
}

Ray spawn_ray(in SurfaceInteraction si, vec3 wo){
    return Ray(si.p, wo, 0.001, 10000.);
}

void finalize_surface_interaction(inout SurfaceInteraction si, in Ray ray){
    Instance instance = instances[si.instance];
    Material material = materials[instance.material];
    Mesh mesh = meshes[instance.mesh];

    uvec3 triangle = uvec3(indices[mesh.indices + 3 * si.primitive + 0],
                           indices[mesh.indices + 3 * si.primitive + 1],
                           indices[mesh.indices + 3 * si.primitive + 2]);
    
    vec3 p0 = (instance.to_world * vec4(positions[mesh.positions + triangle.x], 1.)).xyz;
    vec3 p1 = (instance.to_world * vec4(positions[mesh.positions + triangle.y], 1.)).xyz;
    vec3 p2 = (instance.to_world * vec4(positions[mesh.positions + triangle.z], 1.)).xyz;

    si.p = p0 * si.barycentric.x + p1 * si.barycentric.y + p2 * si.barycentric.z;
    
    si.n = normalize(cross(p1 - p0, p2 - p0));
    
    // vec3 n0 = normals[mesh.normals + triangle.x];
    // vec3 n1 = normals[mesh.normals + triangle.y];
    // vec3 n2 = normals[mesh.normals + triangle.z];
    //
    // vec3 n = n0 * si.barycentric.x + n1 * si.barycentric.y + n2 * si.barycentric.z;
    // si.n = normalize(inverse(transpose(mat3(instance.to_world))) * n);

    vec2 uv0 = uvs[mesh.uvs + triangle.x];
    vec2 uv1 = uvs[mesh.uvs + triangle.y];
    vec2 uv2 = uvs[mesh.uvs + triangle.z];

    vec2 uv = uv0 * si.barycentric.x + uv1 * si.barycentric.y + uv2 * si.barycentric.z;
    si.uv = uv;
        
    mat3 tbn = compute_TBN(uv1 - uv0, uv2 - uv0, p1 - p0, p2 - p0, si.n);
    si.tbn = tbn;

    si.material = material;

    si.wi = to_local(si, -ray.d);
}

#endif //UTILS_GLSL
