#ifndef EMITTER_GLSL
#define EMITTER_GLSL

#include "interaction.glsl"
#include "records.glsl"
#include "instance.glsl"

void sample_direction(
    in Emitter emitter, 
    in SurfaceInteraction si, 
    vec2 sample1, 
    out DirectionSample ds, 
    out vec3 val){
    if (emitter.ty == EMITTER_TY_AREA){
        Instance instance = instances[emitter.instance];
        
        PositionSample ps = sample_position(instance, sample1);
        // //DEBUG:
        // imageStore(image[0], ivec2(gl_LaunchIDEXT.xy), vec4(ps.uv, 0., 1.));
        
        ds = direction_sample(ps);

        //DEBUG:
        
        ds.d = ds.p - si.p;

        float dist2 = dot(ds.d, ds.d);
        ds.dist = sqrt(dist2);
        ds.d /= ds.dist;

        float dp = abs(dot(ds.d, ds.n));
        
        ds.pdf = (dp > 0.)?dist2/dp:0.;

        //Material material = materials[instance.material];
        val = eval_texture(emitter.emission, ds.uv);
    } else{
        val = vec3(0.);
        ds.pdf = 0.;
    }
}

float pdf_emitter(uint emitter){
    return 1. / float(emitters.length());
}

// void sample_emitter(float sample1, out uint emitter, out float weight, out float sample_reuse){
//     float index_sample_scaled = sample1 * float(emitters.length());
//     uint index = uint(index_sample_scaled);
//     
//     //emitter = emitters[index];
//     emitter = index;
//     weight = float(emitters.length());
//     sample_reuse = index_sample_scaled - float(index);
// }

void sample_emitter_direction(
    in SurfaceInteraction si, 
    vec2 sample1, 
    out DirectionSample ds, 
    out vec3 val){
    
    float emitter_weight = 1. / float(emitters.length());
    uint emitter_idx = sample_reuse(sample1.x, emitters.length());

    Emitter emitter = emitters[emitter_idx];
    sample_direction(emitter, si, sample1, ds, val);

    ds.pdf *= pdf_emitter(emitter_idx);
    
    val *= emitter_weight * abs(dot(ds.n, ds.d));

    bool occluded = ray_test(spawn_ray_to(si, ds.p));
    if (occluded){
        ds.pdf = 0.;
        val = vec3(0.);
    }
}

float pdf_emitter_direction(in SurfaceInteraction si){
    Instance instance = instances[si.instance];
    if (instance.emitter >= 0 && abs(cos_theta(si.wi)) > 0.){
        Mesh mesh = meshes[instance.mesh];
        
        float pdf = (si.dist * si.dist) / abs(cos_theta(si.wi));
        pdf *= pdf_emitter(instance.emitter);
        
        pdf *= square_to_uniform_triangle_pdf(si.barycentric.yz);
        pdf *= 1. / si.area;
        pdf *= 1. / float(mesh.indices_count / 3);
        return pdf;
    }else{
        return 0;
    }
}

#endif //EMITTER_GLSL
