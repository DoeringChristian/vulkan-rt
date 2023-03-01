#version 460

layout(set = 0, binding = 0, rgba32f) uniform image2D current;
layout(set = 0, binding = 1, rgba32f) uniform image2D avg;


layout(push_constant) uniform PushConstants{
    uint frame_count;
};

void main(){
    vec4 current_value = imageLoad(current, ivec2(gl_GlobalInvocationID.xy));
    vec4 avg_value = imageLoad(avg, ivec2(gl_GlobalInvocationID.xy));

    if (frame_count == 0){
        avg_value = current_value;
    }else{
        avg_value = avg_value * (1 - 1/float(frame_count)) + current_value / float(frame_count);
    }
    imageStore(avg, ivec2(gl_GlobalInvocationID.xy), avg_value);
}

