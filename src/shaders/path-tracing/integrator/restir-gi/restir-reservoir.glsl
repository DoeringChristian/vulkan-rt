#ifndef RESTIR_RESERVOIR_GLSL
#define RESTIR_RESERVOIR_GLSL

void update(inout RestirReservoir self, RestirSample snew, float wnew, float sample1d){
    self.w = self.w + wnew;
    self.M += 1;
    if (sample1d < wnew/self.w){
        self.z = snew;
    }
}

void merge(inout RestirReservoir self, const RestirReservoir r, float p_hat, float sample1d){
    uint M_0 = self.M;
    update(self, r.z, p_hat * r.W * r.M, sample1d);
    self.M = M_0 + r.M;
}

void init(out RestirReservoir self){
    self.W = 0;
    self.w = 0;
    self.M = 0;
}

void init(out RestirSample self){
    self.x_v = vec3(0);
    self.n_v = vec3(0);
    self.x_s = vec3(0);
    self.n_s = vec3(0);
    
    self.L_o = vec3(0);
    self.f = vec3(0);
    self.p_q = 0;
}

#endif //RESTIR_RESERVOIR_GLSL
