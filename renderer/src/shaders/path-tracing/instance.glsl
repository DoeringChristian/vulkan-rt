#ifndef MESH_GLSL
#define MESH_GLSL

#include "interaction.glsl"
#include "warp.glsl"

float sample_position_pdf(in Instance instance, in PositionSample ps){
    Mesh mesh = meshes[instance.mesh];
    float pdf = 1.;
    pdf *= 1. / ps.area;
    pdf *= 1. / float(mesh.indices_count / 3);
    pdf *= square_to_uniform_triangle_pdf(ps.barycentric.yz);
    return pdf;
}
float sample_position_pdf(in Instance instance, in SurfaceInteraction si){
    Mesh mesh = meshes[instance.mesh];
    float pdf = 1.;
    pdf *= 1. / si.area;
    pdf *= 1. / float(mesh.indices_count / 3);
    pdf *= square_to_uniform_triangle_pdf(si.barycentric.yz);
    return pdf;
}

PositionSample sample_position(in Instance instance, vec2 sample1){
    PositionSample ps;
    Mesh mesh = meshes[instance.mesh];
    
    uint primitive_count = mesh.indices_count / 3;
    
    // //DEBUG:
    // imageStore(image[0], ivec2(gl_LaunchIDEXT.xy), vec4(instance.mesh, mesh.indices_count, primitive_count, 0.));
        
    uint primitive = sample_reuse(sample1.x, primitive_count);
    ps.pdf = 1./float(primitive_count);
    
    vec2 b = square_to_uniform_triangle(sample1);
    ps.pdf *= square_to_uniform_triangle_pdf(b);
    
    vec3 barycentric = vec3((1. - b.x -b.y), b.x, b.y);

    ps.barycentric = barycentric;

    // Same as in finalize_surface_interaction
    
    uvec3 triangle = uvec3(indices[mesh.indices + 3 * primitive + 0],
                           indices[mesh.indices + 3 * primitive + 1],
                           indices[mesh.indices + 3 * primitive + 2]);
    
    vec3 p0 = (instance.to_world * vec4(positions[mesh.positions + triangle.x], 1.)).xyz;
    vec3 p1 = (instance.to_world * vec4(positions[mesh.positions + triangle.y], 1.)).xyz;
    vec3 p2 = (instance.to_world * vec4(positions[mesh.positions + triangle.z], 1.)).xyz;

    ps.p = p0 * barycentric.x + p1 * barycentric.y + p2 * barycentric.z;
    
    vec3 n = cross(p1 - p0, p2 - p0);
    ps.area = length(n)/2.;
    ps.pdf *= 1./ps.area;
    ps.n = normalize(n);
    
    vec2 uv0 = uvs[mesh.uvs + triangle.x];
    vec2 uv1 = uvs[mesh.uvs + triangle.y];
    vec2 uv2 = uvs[mesh.uvs + triangle.z];

    vec2 uv = uv0 * ps.barycentric.x + uv1 * ps.barycentric.y + uv2 * ps.barycentric.z;
    ps.uv = uv;
        
    mat3 tbn = compute_TBN(uv1 - uv0, uv2 - uv0, p1 - p0, p2 - p0, ps.n);
    ps.tbn = tbn;

    // //DEBUG:
    // imageStore(image[0], ivec2(gl_LaunchIDEXT.xy), vec4(ps.pdf, 0., 0., 1.));

    // DEBUG:
    //ps.uv = sample1;
    return ps;
}

#endif //MESH_GLSL
