#version 460

layout(location = 0) in vec2 i_uv;

layout(set = 0, binding = 0) uniform sampler2D image_sampler_llr;

layout(location = 0) out vec4 o_color;

vec4 linear_to_srgb(vec4 linear)
{
    bvec4 cutoff = lessThan(linear, vec4(0.0031308));
    vec4 higher = vec4(1.055)*pow(linear, vec4(1.0/2.4)) - vec4(0.055);
    vec4 lower = linear * vec4(12.92);

    return mix(higher, lower, cutoff);
}

void main(){
    //o_color = linear_to_srgb(texture(image_sampler_llr, i_uv));
    vec3 color = texture(image_sampler_llr, i_uv).rgb;
    color = color/(color + vec3(1.));
    color = pow(color, vec3(1./2.2));
    o_color = vec4(color, 1.);
    //o_color = vec4(1., 0., 0., 1.);
}
