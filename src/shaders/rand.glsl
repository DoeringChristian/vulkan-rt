#ifndef RAND_GLSL
#define RAND_GLSL

#define INCREMENTOR 6364136223846793005ul

float uint_to_unit_float(uint u){
    const uint mantissaMask = 0x007FFFFFu;
    const uint one          = 0x3F800000u;
    u &= mantissaMask;
    u |= one;
    float  r2 = uintBitsToFloat(u);
    return r2 - 1.0;
}

uvec2 sample_tea_32(uint v0, uint v1, uint rounds) {
    uint sum = 0;
    for (uint i = 0; i < rounds; ++i) {
        sum += 0x9e3779b9;
        v0 += ((v1 << 4) + 0xa341316c) ^ (v1 + sum) ^ ((v1 >> 5) + 0xc8013ea4);
        v1 += ((v0 << 4) + 0xad90777d) ^ (v0 + sum) ^ ((v0 >> 5) + 0x7e95761e);
    }
    return uvec2(v0, v1);
}

uvec2 sample_tea_32(uint v0, uint v1) {
    return sample_tea_32(v0, v1, 4);
}

// PCG PRNG
uint64_t _state;
uint64_t _inc;

/*
Pcg Hashing algorithm copied from https://www.shadertoy.com/view/XlGcRh.
 https://www.pcg-random.org/
*/
uint pcg(uint v)
{
	uint state = v * 747796405u + 2891336453u;
	uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
	return (word >> 22u) ^ word;
}

void pcg_init(uint64_t seed, uint64_t seq){
    _state = seed;
    _inc = (seq << 1) | 1;
}

void pcg_init(uvec2 seed_seq){
    pcg_init(uint64_t(seed_seq.x), uint64_t(seed_seq.y));
}

uint64_t next_u64(){
    uint64_t old_state = _state;
    _state = old_state * INCREMENTOR + _inc;

    uint64_t xor_shifted = (old_state >> 18) ^ old_state >> 27;

    uint64_t rot = old_state >> 59;
    return (xor_shifted >> rot) | (xor_shifted << ((-rot) & 31));
}

uint next_u32(){
    return uint(next_u64());
}
float next_float(){
    return float(next_u32())/float(0xffffffffu);
}

float next_1d(){
    return next_float();
}
vec2 next_2d(){
    return vec2(next_1d(), next_1d());
}


#endif //RAND_GLSL
