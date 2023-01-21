#ifndef EMITTER_GLSL
#define EMITTER_GLSL

#include "bindings.glsl"
#include "interaction.glsl"
#include "util/instance.glsl"
#include "util/texture.glsl"

Emitter emitter(in SurfaceInteraction si){
    Emitter emitter;
    Instance instance = instances[si.instance];
    if(instance.emitter == -1){
        emitter.ty = EMITTER_TY_NONE;
    }else{
        emitters[instance.emitter];
    }
    return emitter;
}

void emitter_sample_direciton(in Emitter emitter, SurfaceInteraction si, vec2 sample1, out DirectionSample ds, out vec3 val){
    if (emitter.ty == EMITTER_TY_AREA){
        Instance instance = instances[emitter.instance];

        PositionSample ps = instance_sample_position(instance, sample1);
        
        ds.p = ps.p;
        ds.uv = ps.uv;
        ds.n = ps.n;
        ds.pdf = ps.pdf;
        ds.barycentric = ps.barycentric;
        ds.tbn = ps.tbn;
        
        ds.d = ds.p - si.p;

        float dist2 = dot(ds.d, ds.d);
        ds.dist = sqrt(dist2);
        ds.d /= ds.dist;

        float dp = dot(ds.d, ds.n);
        if (dp != 0){
            ds.pdf *= dist2/dp;
        }else{
            ds.pdf = 0;
        }

        //Material material = materials[instance.material];
        val = eval_texture(emitter.emission, ds.uv);
    } 
}

float pdf_emitter(uint emitter){
    return 1. / float(emitters.length());
}

void sample_emitter(float sample1, out uint emitter, out float weight, out float sample_reuse){
    float index_sample_scaled = sample1 * float(emitters.length());
    uint index = uint(index_sample_scaled);
    
    //emitter = emitters[index];
    emitter = index;
    weight = float(emitters.length());
    sample_reuse = index_sample_scaled - float(index);
}

void sample_emitter_direction(in SurfaceInteraction si, vec2 sample1, out DirectionSample ds, out vec3 val){
    
    uint emitter_idx;
    float emitter_weight;
    float sample_x_re;
    sample_emitter(sample1.x, emitter_idx, emitter_weight, sample_x_re);
    sample1.x = sample_x_re;

    Emitter emitter = emitters[emitter_idx];
    emitter_sample_direciton(emitter, si, sample1, ds, val);

    ds.pdf *= pdf_emitter(emitter_idx);
    val *= emitter_weight;

    bool occluded = ray_test(spawn_ray_to(si, ds.p));
    if (occluded){
        ds.pdf = 0.;
        val = vec3(0.);
    }
}

vec3 eval_emitter(in SurfaceInteraction si){
    Instance instance = instances[si.instance];
    if(instance.emitter == -1){
        return vec3(0., 0., 0.);
    }else{
        Emitter emitter = emitters[instance.emitter];
        vec3 irradiance = eval_texture(emitter.emission, si.uv);

        return irradiance;
    }
    return vec3(0., 0., 0.);
}

#endif //EMITTER_GLSL
