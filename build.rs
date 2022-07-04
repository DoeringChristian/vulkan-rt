use spirv_builder::{Capability, MetadataPrintout, SpirvBuilder, SpirvMetadata};

fn main() {
    SpirvBuilder::new("shaders/test01/", "spirv-unknown-vulkan1.2")
        .extension("SPV_KHR_ray_tracing")
        .extension("SPV_KHR_physical_storage_buffer")
        .capability(Capability::RayTracingKHR)
        .capability(Capability::Int64)
        .capability(Capability::PhysicalStorageBufferAddresses)
        .print_metadata(MetadataPrintout::Full)
        .spirv_metadata(SpirvMetadata::Full)
        .preserve_bindings(true)
        .build()
        .unwrap();
}
