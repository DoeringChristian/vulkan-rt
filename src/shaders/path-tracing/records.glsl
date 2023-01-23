#ifndef RECORDS_GLSL
#define RECORDS_GLSL

struct PositionSample{
    vec3 p;
    vec2 uv;
    vec3 n;
    float pdf;
    float area;

    vec3 barycentric;

    mat3 tbn;
};

struct DirectionSample{
    vec3 p;
    vec2 uv;
    vec3 n;
    float pdf;
    float area;

    vec3 barycentric;

    mat3 tbn;
    //
    
    vec3 d;
    float dist;
};

DirectionSample direction_sample(in SurfaceInteraction si){
    DirectionSample ds;
    ds.p = si.p;
    ds.uv = si.uv;
    ds.n = si.n;
    ds.area = si.area;
    ds.barycentric = si.barycentric;
    ds.tbn = si.tbn;
    ds.d = -to_world(si, si.wi);
    ds.dist = si.dist;
    return ds;
}

DirectionSample direction_sample(in PositionSample ps){
    DirectionSample ds;
    ds.p = ps.p;
    ds.uv = ps.uv;
    ds.n = ps.n;
    ds.pdf = ps.pdf;
    ds.barycentric = ps.barycentric;
    ds.tbn = ps.tbn;
    return ds;
}

#endif //RECORDS_GLSL
