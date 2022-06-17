
#version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable

#define M_PI 3.1415926535897932384626433832795

struct Material {
    vec4 diffuse;
    vec4 mra;
};

struct Attribute{
    uint mat_index;
    uint model_id;
};

hitAttributeEXT vec2 hit_co;

layout(location = 0) rayPayloadInEXT Payload {
    vec3 orig;
    vec3 dir;
    //vec3 prev_norm;

    vec3 color;
    int depth;

    int ray_active;
} payload;

layout(set = 0, binding = 0) uniform accelerationStructureEXT topLevelAS;
layout(set = 0, binding = 1, rgba32f) uniform image2D image;
layout(set = 0, binding = 2) buffer Attributes{
    Attribute attributes[];
};
layout(set = 0, binding = 3) buffer Materials{
    Material materials[];
};
layout(set = 0, binding = 4) buffer Indices{
    uint indices[];
}model_indices[];
layout(set = 0, binding = 5) buffer Positions{
    float positions[];
}model_positions[];

float rand(float seed){
    return fract(sin(seed * 12.9898) * 43758.5453);
}

float rand(vec2 seed) {
    return fract(sin(dot(seed, vec2(12.9898, 78.233))) * 43758.5453);
}

float rand(vec3 seed){
    return fract(sin(dot(seed, vec3(12.9898, 78.233, 43.5295935))) * 43758.5453);
}

vec2 rand2(float seed){
    return vec2(rand(seed), rand(seed + 44567.2901));
}
vec2 rand2(vec2 seed){
    return vec2(rand(seed), rand(seed + vec2(63775.8729, 84230.7473)));
}
vec2 rand2(vec3 seed){
    return vec2(rand(seed), rand(seed + vec3(63775.8729, 84230.7473, 54643.5341)));
}


vec3 random_sphere(vec3 seed){
    vec2 uv = rand2(seed);
    float theta = acos(1. - 2. * uv.x);
    float phi = 2 * M_PI *    uv.y;
    return vec3(
        cos(phi) * sin(theta),
        sin(phi) * sin(theta),
        cos(theta)
    );
}
vec3 random_hemisphere(vec3 normal, vec3 seed){
    vec3 sphere = random_sphere(seed);
    if (dot(normal , sphere) <= 0.){
        return reflect(sphere, normal);
    }
    return sphere;
}

float NDF_GGXTR(float nh, float roughness){
    float a = roughness * roughness;
    float a2 = a * a;
    float nh2 = nh * nh;
    
    float num = a2;
    float denom = (nh2 * (a2 - 1.) + 1);
    denom = M_PI * denom * denom;
    return num/denom;
}

float GSchlickGGX(float nv, float roughness){
    float r = (roughness + 1.);
    float k = (r*r)/8.;
    
    float num = nv;
    float denom = nv * (1. - k) + k;
    
    return num/denom;
}
float GSmith(float nv, float nl, float roughness){
    float ggx1 = GSchlickGGX(nv, roughness);
    float ggx2 = GSchlickGGX(nl, roughness);
    return ggx1 * ggx2;
}
vec3 FSchlick(float cosTheta, vec3 F0)
{
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}  

void main() {
    if (payload.ray_active == 0) {
        return;
    }

    Attribute attr = attributes[gl_InstanceCustomIndexEXT];
    Material mat = materials[attr.mat_index];
    //uint idx0 = model_indices[attr.model_id].indices[0];

    ivec3 indices = ivec3(model_indices[attr.model_id].indices[3 * gl_PrimitiveID + 0],
                        model_indices[attr.model_id].indices[3 * gl_PrimitiveID + 1],
                        model_indices[attr.model_id].indices[3 * gl_PrimitiveID + 2]);

    vec3 barycentric = vec3(1. - hit_co.x - hit_co.y, hit_co.x, hit_co.y);

    vec3 pos0 = vec3(model_positions[attr.model_id].positions[3 * indices.x + 0],
                    model_positions[attr.model_id].positions[3 * indices.x + 1],
                    model_positions[attr.model_id].positions[3 * indices.x + 2]);
    vec3 pos1 = vec3(model_positions[attr.model_id].positions[3 * indices.y + 0],
                    model_positions[attr.model_id].positions[3 * indices.y + 1],
                    model_positions[attr.model_id].positions[3 * indices.y + 2]);
    vec3 pos2 = vec3(model_positions[attr.model_id].positions[3 * indices.z + 0],
                    model_positions[attr.model_id].positions[3 * indices.z + 1],
                    model_positions[attr.model_id].positions[3 * indices.z + 2]);

    vec3 pos = pos0 * barycentric.x + pos1 * barycentric.y + pos2 * barycentric.z;
    vec3 geo_norm = normalize(cross(pos1 - pos0, pos2 - pos0));
    

    payload.orig = vec3(0., 0., 0.);
    payload.dir = vec3(0., 1., 0.);


    //payload.color = mat.diffuse.xyz;
    payload.color = geo_norm;

    //payload.prev_norm = vec3(0., 0., 1.);

    payload.depth += 1;
}
