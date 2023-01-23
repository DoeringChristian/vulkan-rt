#![no_std]
#![cfg_attr(target_arch = "spirv", feature(asm_experimental_arch,))]

use spirv_std::spirv;

#[spirv(ray_generation)]
pub fn ray_trace() {}
