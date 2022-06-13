use bytemuck::cast_slice;
use screen_13::prelude::*;
use std::mem::size_of;
use std::sync::Arc;

pub struct PositionsBuffer {
    pub data: Arc<Buffer>,
    pub count: u64,
    pub stride: u64,
    pub format: vk::Format,
}

impl PositionsBuffer {
    pub fn create(device: &Arc<Device>, positions: &[f32]) -> Self {
        let data = Arc::new({
            let data = cast_slice(positions);
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(
                    data.len() as _,
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });
        let count = positions.len() / 3;
        let stride = 3 * size_of::<f32>();
        let format = vk::Format::R32G32B32_SFLOAT;
        Self {
            data,
            count: count as _,
            stride: stride as _,
            format,
        }
    }
}

pub struct IndexBuffer {
    pub data: Arc<Buffer>,
    pub count: u32,
    pub ty: vk::IndexType,
}

impl IndexBuffer {
    pub fn create(device: &Arc<Device>, indices: &[u32]) -> Self {
        let data = Arc::new({
            let data = cast_slice(indices);
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(
                    data.len() as _,
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                ),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });
        Self {
            data,
            count: indices.len() as _,
            ty: vk::IndexType::UINT32,
        }
    }
}

pub struct InstanceBuffer {
    pub data: Arc<Buffer>,
    pub count: usize,
}

impl InstanceBuffer {
    pub fn create(
        device: &Arc<Device>,
        instances: &[vk::AccelerationStructureInstanceKHR],
    ) -> Self {
        let buf_size = instances.len() * size_of::<vk::AccelerationStructureInstanceKHR>();
        let mut buf = Buffer::create(
            device,
            BufferInfo::new_mappable(
                (size_of::<vk::AccelerationStructureInstanceKHR>() * instances.len()) as _,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            ),
        )
        .unwrap();
        Buffer::copy_from_slice(&mut buf, 0, unsafe {
            std::slice::from_raw_parts(instances as *const _ as *const _, buf_size as _)
        });
        Self {
            data: Arc::new(buf),
            count: instances.len(),
        }
    }
}
