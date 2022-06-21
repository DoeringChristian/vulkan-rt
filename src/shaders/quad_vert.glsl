#version 460

const vec2 POS[6] = {
    vec2(-1., -1.),
    vec2(1., 1.),
    vec2(1., -1.),
    vec2(-1., -1.),
    vec2(-1., 1.),
    vec2(1., 1.),
};

vec2 pos(){
    return POS[gl_VertexIndex];
}
vec2 uv(){
    return (pos() + vec2(1.)) / 2.;
}

layout(location = 0) out vec2 o_uv;

void main(){
    o_uv = uv();
    vec2 pos = pos();
    gl_Position = vec4(pos.x, pos.y, 0., 1.);
}
    
