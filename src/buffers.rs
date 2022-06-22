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
