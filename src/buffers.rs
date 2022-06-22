use bytemuck::cast_slice;
use screen_13::prelude::*;
use std::marker::PhantomData;
use std::mem::size_of;
use std::sync::Arc;

pub struct TypedBuffer<T> {
    pub buf: Arc<Buffer>,
    count: usize,
    _ty: PhantomData<T>,
}

impl<T: Copy + Sized> TypedBuffer<T> {
    pub fn create(device: &Arc<Device>, data: &[T], usages: vk::BufferUsageFlags) -> Self {
        let buf = Arc::new({
            // SAFETY: there is no safty in this. I would love for
            // vk::AccelerationStructureInstanceKHR to implement bytemuck.
            let data = unsafe {
                std::slice::from_raw_parts(
                    data as *const _ as *const _,
                    data.len() * std::mem::size_of::<T>(),
                )
            };
            let mut buf =
                Buffer::create(device, BufferInfo::new_mappable(data.len() as _, usages)).unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });

        Self {
            buf,
            count: data.len(),
            _ty: PhantomData,
        }
    }
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslInstanceData {
    pub mat_index: u32,
    pub indices: u32,
    pub positions: u32,
    //pub _pad: [u32; 2],
}

pub struct InstanceDataBuf {
    pub data: Arc<Buffer>,
    pub count: usize,
}

impl InstanceDataBuf {
    pub fn create(device: &Arc<Device>, attributes: &[GlslInstanceData]) -> Self {
        let buf = Arc::new({
            let data = cast_slice(attributes);
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(data.len() as _, vk::BufferUsageFlags::STORAGE_BUFFER),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });
        let count = attributes.len();

        Self { data: buf, count }
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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlslMaterial {
    pub diffuse: [f32; 4],
    pub mra: [f32; 4],
    pub emission: [f32; 4],
}

pub struct MaterialBuffer {
    pub data: Arc<Buffer>,
    pub count: usize,
}

impl MaterialBuffer {
    pub fn create(device: &Arc<Device>, materials: &[GlslMaterial]) -> Self {
        let buf = Arc::new({
            let data = cast_slice(materials);
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(data.len() as _, vk::BufferUsageFlags::STORAGE_BUFFER),
            )
            .unwrap();
            trace!("data_len: {}", data.len());
            Buffer::copy_from_slice(&mut buf, 0, data);
            buf
        });
        let count = materials.len();

        Self { data: buf, count }
    }
}
