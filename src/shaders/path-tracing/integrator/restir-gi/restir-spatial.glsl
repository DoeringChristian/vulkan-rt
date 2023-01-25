#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_buffer_reference2 : require
#extension GL_EXT_scalar_block_layout: require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require

#include "common.glsl"
#include "scene-bindings.glsl"
#include "restir-pushconstant.glsl"

// Ray Tracing Bindings
layout(location = 0) rayPayloadEXT Payload payload;
layout(location = 1) rayPayloadEXT bool shadow_payload;

layout(std140, set = 1, binding = 0) buffer InitialSamples{
    RestirSample initial_samples[];
};
layout(std140, set = 1, binding = 1) buffer TemporalReservoir{
    RestirReservoir temporal_reservoir[];
};
layout(std140, set = 1, binding = 2) buffer SpatialReservoir{
    RestirReservoir spatial_reservoir[];
};

#include "trace.glsl"

#include "sampler/independent.glsl"
#include "bsdf/diffuse.glsl"
#include "camera.glsl"
#include "emitter.glsl"

#include "restir-path.glsl"
#include "restir-reservoir.glsl"

#define M_MAX 500

uint pixel_idx = (gl_LaunchIDEXT.y * gl_LaunchSizeEXT.x + gl_LaunchIDEXT.x);
vec2 pixel_pos = vec2(gl_LaunchIDEXT.xy);

float p_hat(const vec3 f){
    return length(f);
}

void combine_reservoir(inout RestirReservoir r1, const RestirReservoir r2, const RestirSample q, const RestirSample q_n, float sample1d, out bool gt){
    const uint r2_m = min(r2.M, M_MAX);
    float r2_hat = p_hat(r2.z.L_o);
    bool visible = !ray_test(ray_from_to(q.x_v, r2.z.x_s));

    if (!visible){
        r2_hat = 0;
    }else{
        vec3 w_qq = q.x_v - q.x_s;
        const float w_qq_len = length(w_qq);
        w_qq /= w_qq_len;
        vec3 w_rq = r2.z.x_v - q.x_s;
        const float w_rq_len = length(w_rq);
        w_rq /= w_rq_len;
        const float qq = w_qq_len * w_qq_len;
        const float rq = w_rq_len * w_rq_len;
        const float div = rq * abs(dot(w_qq, q.n_s));
        const float j = div == 0 ? 0 : abs(dot(w_rq, q.n_s)) * qq / div;
        r2_hat *= j;
    }

    const float factor = r2_hat * r2_m * r2.W;
    gt = factor > 0;
    if (factor > 0){
        update(r1, r2.z, factor, sample1d);
    }
}


void main(){
    const float max_r = 3;
    const float dist_threshold = 0.01;
    const float angle_threshold = 25 * PI / 180;
    
    SampleGenerator sample_generator = sample_generator(push_constant.seed, pixel_idx); // TODO: maybe init from sample
    
    RestirReservoir R_s = spatial_reservoir[pixel_idx];

    const uint max_iter = R_s.M < M_MAX / 2 ? 9 : 3;
    vec3 Q[9] = vec3[9](vec3(0), vec3(0), vec3(0), vec3(0), vec3(0), vec3(0),
                        vec3(0), vec3(0), vec3(0));
    uint Q_h[9] = uint[9](0, 0, 0, 0, 0, 0, 0, 0, 0);
    uint q_cnt = 0;

    RestirReservoir R;
    R.w = 0;
    R.W = 0;
    R.M = 0;
    float factor = push_constant.do_spatiotemporal == 0 ? 0 : R_s.M * R_s.W * p_hat(R_s.z.L_o);
    if (factor > 0){
        update(R, R_s.z, factor, next_1d(sample_generator));
    }
    

    RestirSample q = initial_samples[pixel_idx];
    RestirSample q_n;

    uint Z = R_s.M;
    uint sum = R_s.M;

    for (uint i = 0; i < max_iter; i++){
        // Chose neighbor pixel;
        ivec2 offset = ivec2(square_to_uniform_disk_concentric(next_2d(sample_generator)) * max_r);
        const ivec2 coords = clamp(ivec2(pixel_pos) + offset, ivec2(0), ivec2(gl_LaunchSizeEXT.xy)-1);
        const uint coords_idx = coords.y * gl_LaunchSizeEXT.x + coords.x;

        q_n = initial_samples[coords_idx];

        if (length(q_n.n_s) == 0){
            continue;
        }

        // Geometric Similarity
        float dist = dot(q_n.x_v - q.x_v, q_n.x_v - q.x_v);
        if (dist > dist_threshold || (dot(q_n.n_v, q.n_v)) < cos(angle_threshold)){
            continue;
        }

        RestirReservoir R_n = temporal_reservoir[coords_idx];
        bool gt;
        combine_reservoir(R, R_n, q, q_n, next_1d(sample_generator), gt);
        Q_h[q_cnt] = R_n.M;
        Q[q_cnt++] = q_n.x_s;
        sum += R_n.M;
    }

    const float phat_val = p_hat(R.z.L_o);
    if (phat_val > 0){
        for (uint i = 0; i< q_cnt; i++){

            vec3 dir = Q[i] - R.z.x_v;
            float len = length(dir);
            dir /= len;

            bool shadowed = ray_test(ray_from_to(R.z.x_v, Q[i]));
            if (!shadowed){
                Z += Q_h[i];
            }
        }
    }

    R.M = min(sum, M_MAX);
    R.W = Z * phat_val == 0 ? 0 : R.w / (Z * phat_val);
    spatial_reservoir[pixel_idx] = R;
}
