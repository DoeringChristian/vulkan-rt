#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "rand.glsl"
#include "bindings.glsl"
#include "common.glsl"
#include "utils.glsl"
#include "diffuse_bsdf.glsl"

const uint min_rr = 2;

void main() {

    uint idx = uint(gl_LaunchSizeEXT.x * gl_LaunchIDEXT.y + gl_LaunchIDEXT.x);

    pcg_init(sample_tea_32(push_constant.seed, idx));

    vec2 pos = gl_LaunchIDEXT.xy;
    vec2 sample_pos = pos + next_2d();
    vec2 adjusted_pos = sample_pos / gl_LaunchSizeEXT.xy;

    Camera camera = cameras[push_constant.camera];

    mat4 view_to_camera = inverse(camera.to_view);

    vec3 near_p = (view_to_camera * vec4(adjusted_pos.xy, 0., 1.)).xyz;

    
    vec3 o = camera.to_world[3].xyz;
    vec3 d = normalize(near_p);
    
    Ray ray;
    
    ray.o = o;
    ray.d = -normalize((camera.to_world * vec4(d, 0.))).xyz;
    
    float near_t = camera.near_clip / -d.z;
    float far_t = camera.far_clip / -d.z;

    ray.tmin = near_t;
    ray.tmax = far_t;
    
    
    vec3 L = vec3(0.);
    vec3 f = vec3(1.);
    uint depth = 0;
    
    // DEBUG:
    
    payload.valid = 0;
    SurfaceInteraction si;

    bool ray_active = true;

    while (depth < push_constant.max_depth && ray_active){
        traceRayEXT(accel, gl_RayFlagsOpaqueEXT, 0xFF, 0, 0, 0,
                    ray.o, 0.001, ray.d, 10000.0, 0);


        if (payload.valid == 0){
            ray_active = false;
            break;
        }
        
        // DEBUG:
        // L = ray.d;
        // break;
        
        si.instance = payload.instance;
        si.primitive = payload.primitive;
        si.valid = payload.valid;
        si.barycentric = payload.barycentric;
        
        finalize_surface_interaction(si, ray);
        
        // DEBUG:


        BSDFSample bs;
        vec3 bsdf_value;
        sample_bsdf(si, next_1d(), next_2d(), bs, bsdf_value);
        

        L += f * eval_texture(si.material.emission, si);
        f *= bsdf_value;

        ray = spawn_ray(si, to_world(si, bs.wo));
        
        //===========================================================
        // Throughput Russian Roulette:
        //===========================================================
        float f_max = max(f.r, max(f.g, f.b));
        float rr_prop = f_max;
        
        if (depth < push_constant.rr_depth){
            rr_prop = 1.;
        }
        f *= 1. / rr_prop;
        bool rr_continue = next_float() < rr_prop;
        if (!rr_continue){
            ray_active = false;
            break;
        }

        depth += 1;

        // DEBUG:
        //L = vec3(si.uv, 0.);
        //L = eval_texture(si.material.base_color, si);
        //break;
    }

    imageStore(image, ivec2(gl_LaunchIDEXT.xy), vec4(L, 1.));
}
