#ifndef BINDINGS_HLSL
#define BINDINGS_HLSL

#include "common.hlsl"

[[vk::binding(0, 0)]] RaytracingAccelerationStructure accel;
[[vk::binding(1, 0)]] RWTexture2D<float4> image[];

[[vk::binding(0, 1)]] StructuredBuffer<uint> indices;
[[vk::binding(0, 2)]] StructuredBuffer<float3> positions;
[[vk::binding(0, 3)]] StructuredBuffer<float3> normals;
[[vk::binding(0, 4)]] StructuredBuffer<float2> uvs;

[[vk::binding(0, 5)]] StructuredBuffer<Instance> instances;
[[vk::binding(0, 6)]] StructuredBuffer<Mesh> Meshes;
[[vk::binding(0, 7)]] StructuredBuffer<Emitter> emitters;
[[vk::binding(0, 8)]] StructuredBuffer<Material> materials;
[[vk::binding(0, 9)]] StructuredBuffer<Camera> cameras;

[[vk::binding(0, 10)]] Texture2D<float4> textures[];

[[vk::push_constant]]
struct {
    uint camera;
    uint max_depth;
    uint rr_depth;
    uint seed;
} push_constants;

#endif //BINDINGS_HLSL
