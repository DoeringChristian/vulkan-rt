use screen_13::prelude::*;
use std::sync::Arc;

fn align_up(val: u32, atom: u32) -> u32 {
    (val + atom - 1) & !(atom - 1)
}

pub struct SbtBufferInfo<'a> {
    pub rgen_index: usize,
    pub hit_indices: &'a [usize],
    pub miss_indices: &'a [usize],
    pub callable_indices: &'a [usize],
}

pub struct SbtBuffer {
    buffer: Arc<Buffer>,
    sbt_rgen: vk::StridedDeviceAddressRegionKHR,
    sbt_miss: vk::StridedDeviceAddressRegionKHR,
    sbt_hit: vk::StridedDeviceAddressRegionKHR,
    sbt_callable: vk::StridedDeviceAddressRegionKHR,
}

impl SbtBuffer {
    pub fn buffer(&self) -> &Arc<Buffer> {
        &self.buffer
    }
    pub fn rgen(&self) -> vk::StridedDeviceAddressRegionKHR {
        self.sbt_rgen
    }
    pub fn miss(&self) -> vk::StridedDeviceAddressRegionKHR {
        self.sbt_miss
    }
    pub fn hit(&self) -> vk::StridedDeviceAddressRegionKHR {
        self.sbt_hit
    }
    pub fn callable(&self) -> vk::StridedDeviceAddressRegionKHR {
        self.sbt_callable
    }
    pub fn create<'a>(
        device: &Arc<Device>,
        info: SbtBufferInfo<'a>,
        pipeline: &RayTracePipeline,
    ) -> Result<Self, DriverError> {
        let &PhysicalDeviceRayTracePipelineProperties {
            shader_group_base_alignment,
            shader_group_handle_alignment,
            shader_group_handle_size,
            ..
        } = device
            .ray_tracing_pipeline_properties
            .as_ref()
            .ok_or(DriverError::Unsupported)?;

        let sbt_handle_size = align_up(shader_group_handle_size, shader_group_handle_alignment);
        let sbt_rgen_size = align_up(sbt_handle_size, shader_group_base_alignment);
        let sbt_hit_size = align_up(
            info.hit_indices.len() as u32 * sbt_handle_size,
            shader_group_base_alignment,
        );
        let sbt_miss_size = align_up(
            info.miss_indices.len() as u32 * sbt_handle_size,
            shader_group_base_alignment,
        );
        let sbt_callable_size = align_up(
            info.callable_indices.len() as u32 * sbt_handle_size,
            shader_group_base_alignment,
        );
        trace!(
            "shader_group_base_alignment: {}",
            shader_group_base_alignment
        );

        let buffer = Arc::new({
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(
                    (sbt_rgen_size + sbt_hit_size + sbt_miss_size + sbt_callable_size) as _,
                    vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                ),
            )
            .unwrap();

            let mut data = Buffer::mapped_slice_mut(&mut buf);
            data.fill(0);

            let rgen_handle = pipeline.group_handle(info.rgen_index)?;
            data[0..rgen_handle.len()].copy_from_slice(rgen_handle);
            data = &mut data[sbt_rgen_size as _..];

            for idx in info.hit_indices {
                let handle = pipeline.group_handle(*idx)?;
                data[0..handle.len()].copy_from_slice(handle);
                data = &mut data[sbt_hit_size as _..];
            }

            //trace!("info.miss_indices: {}", info.miss_indices.len());
            for idx in info.miss_indices {
                let handle = pipeline.group_handle(*idx)?;
                data[0..handle.len()].copy_from_slice(handle);
                //trace!("miss_sbt: {:#?}", data);
                data = &mut data[sbt_hit_size as _..];
            }
            buf
        });

        let mut address = Buffer::device_address(&buffer);
        let sbt_rgen = vk::StridedDeviceAddressRegionKHR {
            device_address: address,
            stride: sbt_rgen_size as _,
            size: sbt_rgen_size as _,
        };
        address += sbt_rgen_size as vk::DeviceAddress;
        let sbt_hit = if !info.hit_indices.is_empty() {
            vk::StridedDeviceAddressRegionKHR {
                device_address: address,
                stride: sbt_handle_size as _,
                size: sbt_hit_size as _,
            }
        } else {
            vk::StridedDeviceAddressRegionKHR::default()
        };
        address += sbt_hit_size as vk::DeviceAddress;
        let sbt_miss = if !info.miss_indices.is_empty() {
            vk::StridedDeviceAddressRegionKHR {
                device_address: address,
                stride: sbt_handle_size as _,
                size: sbt_miss_size as _,
            }
        } else {
            vk::StridedDeviceAddressRegionKHR::default()
        };
        address += sbt_miss_size as vk::DeviceAddress;
        let sbt_callable = if !info.callable_indices.is_empty() {
            vk::StridedDeviceAddressRegionKHR {
                device_address: address,
                stride: sbt_handle_size as _,
                size: sbt_callable_size as _,
            }
        } else {
            vk::StridedDeviceAddressRegionKHR::default()
        };

        Ok(Self {
            buffer,
            sbt_rgen,
            sbt_hit,
            sbt_miss,
            sbt_callable,
        })
    }
}
