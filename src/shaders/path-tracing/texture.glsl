#ifndef TEXTURE_GLSL
#define TEXTURE_GLSL

vec3 eval_texture(in Texture tex, vec2 uv){
    if (tex.ty == TEXTURE_TY_CONSTANT){
        return tex.val;
    }else if(tex.ty == TEXTURE_TY_IMAGE){
        return texture(textures[tex.texture], uv).rgb;
    }
    return vec3(0.);
}

vec2 texture_sample_position(in Texture tex, vec2 sample1){
    return sample1;
}

#endif //TEXTURE_GLSL
