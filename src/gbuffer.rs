use screen_13::prelude::*;
use std::sync::Arc;

pub struct GBuffer {
    pub color: Arc<Image>,
    pub size: [usize; 2],
}

impl GBuffer {
    pub fn new(device: &Arc<Device>, size: [usize; 2]) -> Self {
        let color = Arc::new(
            Image::create(
                device,
                ImageInfo::new_2d(
                    vk::Format::R32G32B32A32_SFLOAT,
                    size[0] as _,
                    size[1] as _,
                    vk::ImageUsageFlags::STORAGE
                        | vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::TRANSFER_DST
                        | vk::ImageUsageFlags::SAMPLED,
                ),
            )
            .unwrap(),
        );
        Self { color, size }
    }
}
