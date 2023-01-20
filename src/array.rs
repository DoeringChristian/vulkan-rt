use crevice::std140::WriteStd140;
use screen_13::prelude::*;
use std::marker::PhantomData;
use std::mem::size_of;
use std::sync::Arc;

pub struct Array<T> {
    pub buf: Arc<Buffer>,
    count: usize,
    stride: usize,
    _ty: PhantomData<T>,
}
impl<T> Array<T> {
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }
    #[inline]
    pub fn stride(&self) -> usize {
        self.stride
    }
}

impl Array<u8> {
    pub fn from_slice_u8(device: &Arc<Device>, usage: vk::BufferUsageFlags, data: &[u8]) -> Self {
        let buf = Arc::new(Buffer::create_from_slice(device, usage, &data).unwrap());
        Self {
            buf,
            stride: size_of::<u8>(),
            count: data.len(),
            _ty: PhantomData,
        }
    }
}

impl<T: crevice::std140::WriteStd140 + Sized + crevice::std140::AsStd140> Array<T> {
    pub fn storage(device: &Arc<Device>, data: &[T]) -> Self {
        Self::from_slice(device, vk::BufferUsageFlags::STORAGE_BUFFER, data)
    }
    pub fn from_slice(device: &Arc<Device>, usage: vk::BufferUsageFlags, data: &[T]) -> Self {
        let buf = Arc::new({
            let mut v = Vec::with_capacity(data.std140_size());
            let mut writer = crevice::std140::Writer::new(&mut v);
            writer.write(data).unwrap();

            let buf = Buffer::create_from_slice(device, usage, &v).unwrap();
            buf
        });

        Self {
            buf,
            stride: T::std140_size_static(),
            count: data.len(),
            _ty: PhantomData,
        }
    }
}

//
// A buffer that prepends the size of the slice as the first element of a uvec4.
// Not a safe way to do this but I have no other idea at the moment
//
pub struct SliceBuffer<T> {
    pub buf: Arc<Buffer>,
    count: usize,
    _ty: PhantomData<T>,
}
impl<T: Copy + Sized> SliceBuffer<T> {
    pub fn create(device: &Arc<Device>, data: &[T], usages: vk::BufferUsageFlags) -> Self {
        let buf = Arc::new({
            // SAFETY: there is no safty in this. I would love for
            // vk::AccelerationStructureInstanceKHR to implement bytemuck.
            let count = [data.len() as u32, 0, 0, 0];
            let count_slice = unsafe {
                std::slice::from_raw_parts(
                    &count as *const _ as *const _,
                    std::mem::size_of::<[u32; 4]>(),
                )
            };
            let data = unsafe {
                std::slice::from_raw_parts(
                    data as *const _ as *const _,
                    data.len() * std::mem::size_of::<T>(),
                )
            };
            let mut buf = Buffer::create(
                device,
                BufferInfo::new_mappable(
                    count_slice.len() as vk::DeviceSize + data.len() as vk::DeviceSize,
                    usages,
                ),
            )
            .unwrap();
            Buffer::copy_from_slice(&mut buf, 0, count_slice);
            if data.len() > 0 {
                Buffer::copy_from_slice(&mut buf, count_slice.len() as _, data);
            }
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
