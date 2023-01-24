#ifndef SPECTRUM_GLSL
#define SPECTRUM_GLSL

float luminance(vec3 c){
    return 0.21271 * c.r + 0.715160 * c.g + 0.072169 * c.b;
}

#endif //SPECTRUM_GLSL
