use crate::accel::{Blas, Tlas};
use crate::array::Array;
use crate::glsl::*;
use glam::*;
use screen_13::prelude::*;
use screen_13_fx::ImageLoader;
use std::sync::Arc;

#[derive(Default)]
pub struct Scene {
    //pub device: Arc<Device>,
    pub indices: Vec<u32>,
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub textures: Vec<image::DynamicImage>,

    pub instances: Vec<Instance>,
    pub meshes: Vec<Mesh>,
    pub emitters: Vec<Emitter>,
    pub materials: Vec<Material>,
    pub cameras: Vec<Camera>,

    // Components on GPU
    pub blases: Vec<Blas<Vec3>>,
    pub tlas: Option<Tlas>,

    pub instance_data: Option<Array<Instance>>,
    pub mesh_data: Option<Array<Mesh>>,
    pub emitter_data: Option<Array<Emitter>>,
    pub material_data: Option<Array<Material>>,
    pub camera_data: Option<Array<Camera>>,

    pub index_data: Option<Array<u32>>,
    pub position_data: Option<Array<Vec3>>,
    pub normal_data: Option<Array<Vec3>>,
    pub uv_data: Option<Array<Vec2>>,

    pub textures_gpu: Option<Vec<Arc<Image>>>,
}

impl Scene {
    pub fn upload(&mut self, device: &Arc<Device>) {
        self.index_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            &self.indices,
        ));
        self.position_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            &self.positions,
        ));
        self.normal_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &self.normals,
        ));
        self.uv_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &self.uvs,
        ));

        self.instance_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &self.instances,
        ));
        self.mesh_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &self.meshes,
        ));
        self.emitter_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &self.emitters,
        ));
        self.material_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &self.materials,
        ));
        self.camera_data = Some(Array::from_slice(
            &device,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            &self.cameras,
        ));

        self.textures_gpu = Some(vec![]);
        for texture in self.textures.iter() {
            let mut img_loader = ImageLoader::new(device).unwrap();
            let img = texture.as_rgba8().unwrap();
            let img = img_loader
                .decode_linear(
                    0,
                    img,
                    screen_13_fx::ImageFormat::R8G8B8A8,
                    img.width(),
                    img.height(),
                )
                .unwrap();
            self.textures_gpu.as_mut().unwrap().push(img);
        }
    }
    pub fn update(&mut self, device: &Arc<Device>, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        // Upload to gpu
        self.upload(device);
        // Create blases
        for instance in self.instances.iter() {
            let mesh = &self.meshes[instance.mesh as usize];
            self.blases.push(Blas::create(
                &device,
                self.index_data.as_ref().unwrap(),
                mesh.indices as usize,
                mesh.indices_count as usize / 3,
                self.position_data.as_ref().unwrap(),
                mesh.positions as usize,
            ))
        }
        // Transform instances into AccelerationStructureInstanceKHR types
        let instances = self
            .instances
            .iter()
            .enumerate()
            .map(|(i, instance)| vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR {
                    matrix: [
                        instance.to_world.x_axis.x,
                        instance.to_world.y_axis.x,
                        instance.to_world.z_axis.x,
                        instance.to_world.w_axis.x,
                        instance.to_world.x_axis.y,
                        instance.to_world.y_axis.y,
                        instance.to_world.z_axis.y,
                        instance.to_world.w_axis.y,
                        instance.to_world.x_axis.z,
                        instance.to_world.y_axis.z,
                        instance.to_world.z_axis.z,
                        instance.to_world.w_axis.z,
                    ],
                },
                instance_custom_index_and_mask: vk::Packed24_8::new(i as _, 0xff),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                ),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: AccelerationStructure::device_address(&self.blases[i].accel),
                },
            })
            .collect::<Vec<_>>();

        // Create tlas from instances
        self.tlas = Tlas::create(&device, &instances);

        // Build blas and tlas
        let blas_nodes = self
            .blases
            .iter()
            .map(|blas| {
                blas.build(cache, rgraph);
                AnyAccelerationStructureNode::AccelerationStructure(rgraph.bind_node(&blas.accel))
            })
            .collect::<Vec<_>>();
        self.tlas
            .as_ref()
            .unwrap()
            .build(cache, rgraph, &blas_nodes);
    }
}
