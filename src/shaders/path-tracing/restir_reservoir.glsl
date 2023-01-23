#ifndef RESTIR_RESERVOIR_GLSL
#define RESTIR_RESERVOIR_GLSL

void update(in RestirReservoir self, RestirSample snew, float wnew, float sample1d){
    self.w = self.w + wnew;
    self.M += 1;
    if sample1d < wnew/self.w{
        self.z = snew;
    }
}

void merge(in RestirReservoir self, RestirReservoir r, float p_hat){
    uint M_0 = self.M;
    update(self, r.z, p_hat * r.W * r.M);
    self.M = M_0 + r.M;
}

#endif //RESTIR_RESERVOIR_GLSL
