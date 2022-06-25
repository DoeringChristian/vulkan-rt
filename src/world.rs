use crate::accel::{Blas, BlasInfo, Tlas};
use crate::buffers::TypedBuffer;
use crate::model::{
    Camera, GlslCamera, GlslInstanceData, GlslMaterial, Index, InstanceBundle, Material,
    MaterialId, MeshId, Texture, TextureId, Vertex, VertexData, Vertices,
};

use bevy_ecs::prelude::*;
use bevy_ecs::system::CommandQueue;
use bevy_math::{Mat3, Mat4, Vec3, Vec4, Vec4Swizzles};
use bevy_transform::prelude::*;
use bytemuck::cast_slice;
use image::GenericImageView;
use screen_13::prelude::*;
use screen_13_fx::ImageLoader;
use slotmap::*;
use std::collections::{BTreeMap, HashMap};
use std::io::BufReader;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

const INDEX_UNDEF: u32 = 0xffffffff;

#[derive(Debug, Clone, Copy)]
pub enum UpdateState {
    Updated,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GpuMeshId {
    pub indices: usize,
    pub vertices: usize,
    pub has_normal: bool,
    pub has_uv0: bool,
    pub has_uv1: bool,
    pub blas: usize,
    pub state: UpdateState,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GpuCameraId {
    pub state: UpdateState,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct GpuTextureId {
    pub tex: usize,
    pub state: UpdateState,
}

pub struct GpuScene {
    pub blases: Vec<Blas>,
    pub tlas: Tlas,

    pub material_buf: TypedBuffer<GlslMaterial>,
    pub instancedata_buf: TypedBuffer<GlslInstanceData>,
    pub indices_bufs: Vec<Arc<TypedBuffer<Index>>>,
    pub vertices_bufs: Vec<Arc<TypedBuffer<Vertex>>>,

    pub textures: Vec<Arc<Image>>,
    pub camera: GlslCamera,
}

impl GpuScene {
    pub fn create(device: &Arc<Device>, scene: &mut Scene) -> Self {
        //let mut positions_bufs = vec![];
        let mut indices_bufs = vec![];
        let mut vertices_bufs = vec![];
        let mut blases = vec![];
        let mut queue = CommandQueue::from_world(&mut scene.world);
        for (entity, indices, vertices) in scene
            .world
            .query::<(Entity, &VertexData<Index>, &Vertices)>()
            .iter(&scene.world)
        {
            let mesh_idx = GpuMeshId {
                indices: indices_bufs.len(),
                vertices: vertices_bufs.len(),
                has_normal: vertices.has_normal,
                has_uv0: vertices.has_uv0,
                has_uv1: vertices.has_uv1,
                blas: blases.len(),
                state: UpdateState::Updated,
            };
            //trace!("positions: {}", positions.0.len());
            vertices_bufs.push(Arc::new(TypedBuffer::create(
                device,
                &vertices.vertices,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            )));
            indices_bufs.push(Arc::new(TypedBuffer::create(
                device,
                &indices.0,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::STORAGE_BUFFER,
            )));
            blases.push(Blas::create(
                device,
                &BlasInfo {
                    indices: indices_bufs.last().unwrap(),
                    positions: vertices_bufs.last().unwrap(),
                },
            ));
            Commands::new(&mut queue, &scene.world)
                .entity(entity)
                .insert(mesh_idx);
            //scene.world.get_entity_mut(entity).unwrap().insert(mesh_idx);
            //trace!("texco: {:#?}", mesh_idx.tex_coords);
            //mesh_idxs.insert(entity, mesh_idx);
        }
        queue.apply(&mut scene.world);
        let mut textures = vec![];
        //let mut textures_idxs = HashMap::new();
        let mut img_loader = ImageLoader::new(device).unwrap();
        for (entity, texture) in scene.world.query::<(Entity, &Texture)>().iter(&scene.world) {
            //textures_idxs.insert(entity, textures.len());
            let tex_id = GpuTextureId {
                tex: textures.len(),
                state: UpdateState::Updated,
            };

            trace!("text: {:#?}", texture.0.color());
            let img = texture.0.as_rgba8().unwrap();
            let img = img_loader
                .decode_linear(
                    img,
                    screen_13_fx::ImageFormat::R8G8B8A8,
                    img.width(),
                    img.height(),
                )
                .unwrap();
            textures.push(img);
            Commands::new(&mut queue, &scene.world)
                .entity(entity)
                .insert(tex_id);
        }
        queue.apply(&mut scene.world);
        let mut materials = vec![];
        let mut material_idxs = HashMap::new();
        for (entity, material) in scene
            .world
            .query::<(Entity, &Material)>()
            .iter(&scene.world)
        {
            let gputex_id = |entity: Entity| -> GpuTextureId {
                *scene
                    .world
                    .get_entity(entity)
                    .unwrap()
                    .get::<GpuTextureId>()
                    .unwrap()
            };
            material_idxs.insert(entity, materials.len());
            materials.push(GlslMaterial {
                albedo: material.albedo,
                mr: material.mr,
                emission: [
                    material.emission[0],
                    material.emission[1],
                    material.emission[2],
                    0.,
                ],
                diffuse_tex: material
                    .albedo_tex
                    .as_ref()
                    .map(|dt| gputex_id(dt.texture).tex)
                    .unwrap_or(INDEX_UNDEF as _) as _,
                mr_tex: material
                    .mr_tex
                    .as_ref()
                    .map(|dt| gputex_id(dt.texture).tex)
                    .unwrap_or(INDEX_UNDEF as _) as _,
                emission_tex: material
                    .emission_tex
                    .as_ref()
                    .map(|dt| gputex_id(dt.texture).tex)
                    .unwrap_or(INDEX_UNDEF as _) as _,
                normal_tex: material
                    .normal_tex
                    .as_ref()
                    .map(|dt| gputex_id(dt.texture).tex)
                    .unwrap_or(INDEX_UNDEF as _) as _,
            });
        }
        let mut instances = vec![];
        //let mut materials = vec![];
        let mut instancedata = vec![];
        for (entity, material_id, mesh_id, transform) in scene
            .world
            .query::<(Entity, &MaterialId, &MeshId, &Transform)>()
            .iter(&scene.world)
        {
            //let mesh_idx = &mesh_idxs[&mesh_id.0];
            let mesh_idx = scene
                .world
                .get_entity(mesh_id.0)
                .unwrap()
                .get::<GpuMeshId>()
                .unwrap();
            let normal_uv_mask: u32 = ((mesh_idx.has_normal as u32) << 0)
                | ((mesh_idx.has_uv0 as u32) << 1)
                | ((mesh_idx.has_uv1 as u32) << 2);
            instancedata.push(GlslInstanceData {
                transform: transform.compute_matrix().to_cols_array_2d(),
                mat_index: material_idxs[&material_id.0] as _,
                vertices: mesh_idx.vertices as _,
                indices: mesh_idx.indices as _,
                normal_uv_mask,
            });
            //trace!("instancedata: {:#?}", instancedata.last());
            //trace!("tex_coords_num: {:#?}", mesh_idx.tex_coords);
            let matrix = transform.compute_matrix();
            let matrix = [
                matrix.x_axis.x,
                matrix.y_axis.x,
                matrix.z_axis.x,
                matrix.w_axis.x,
                matrix.x_axis.y,
                matrix.y_axis.y,
                matrix.z_axis.y,
                matrix.w_axis.y,
                matrix.x_axis.z,
                matrix.y_axis.z,
                matrix.z_axis.z,
                matrix.w_axis.z,
            ];
            instances.push(vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR { matrix },
                instance_custom_index_and_mask: vk::Packed24_8::new(
                    (instancedata.len() - 1) as _,
                    0xff,
                ),
                instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(
                    0,
                    vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() as _,
                ),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: AccelerationStructure::device_address(
                        &blases[mesh_idx.blas].accel,
                    ),
                },
            });
        }
        let camera = {
            let camera = scene
                .world
                .query::<(Entity, &Camera)>()
                .iter(&scene.world)
                .next();
            let camera = if let Some((entity, camera)) = camera {
                Commands::new(&mut queue, &scene.world)
                    .entity(entity)
                    .insert(GpuCameraId {
                        state: UpdateState::Updated,
                    });
                GlslCamera {
                    up: [camera.up[0], camera.up[1], camera.up[2], 1.],
                    right: [camera.right[0], camera.right[1], camera.right[2], 1.],
                    pos: [camera.pos[0], camera.pos[1], camera.pos[2], 1.],
                    focus: camera.focus,
                    diameter: camera.diameter,
                    fov: camera.fov,
                    fc: 0,
                }
            } else {
                GlslCamera {
                    up: [0., 0., 1., 1.],
                    right: [0., 1., 0., 1.],
                    pos: [1., 0., 0., 1.],
                    focus: 1.,
                    diameter: 0.1,
                    fov: 1.,
                    fc: 0,
                }
            };
            camera
        };

        //trace!("instancedata: {:#?}", instancedata);

        let material_buf =
            TypedBuffer::create(device, &materials, vk::BufferUsageFlags::STORAGE_BUFFER);
        let instancedata_buf =
            TypedBuffer::create(device, &instancedata, vk::BufferUsageFlags::STORAGE_BUFFER);

        let tlas = Tlas::create(device, &instances);

        Self {
            blases,
            tlas,
            material_buf,
            instancedata_buf,
            indices_bufs,
            vertices_bufs,
            textures,
            camera,
        }
    }
    pub fn build_accels(&self, cache: &mut HashPool, rgraph: &mut RenderGraph) {
        let blas_nodes = self
            .blases
            .iter()
            .map(|b| b.build(self, cache, rgraph))
            .collect::<Vec<_>>();
        self.tlas.build(self, cache, rgraph, &blas_nodes);
    }
    pub fn update(
        &mut self,
        scene: &mut Scene,
        img: impl Into<AnyImageNode>,
        cache: &mut HashPool,
        rgraph: &mut RenderGraph,
    ) {
        let camera = scene
            .world
            .query::<(&Camera, &GpuCameraId)>()
            .iter(&scene.world)
            .next();
        if let Some((camera, camera_id)) = camera {
            /*
            self.camera = GlslCamera {
                up: [camera.up[0], camera.up[1], camera.up[2], 1.],
                right: [camera.right[0], camera.right[1], camera.right[2], 1.],
                pos: [camera.pos[0], camera.pos[1], camera.pos[2], 1.],
                focus: camera.focus,
                diameter: camera.diameter,
                fov: camera.fov,
                fc: 0,
            };
            */
            //rgraph.clear_color_image(img);
        }
    }
}

// TODO: change add pipeline and sbt to scene.
// where sbt indexes into shader groups of pipeline.
// (maybee sbt should be part of tlas)
pub struct Scene {
    pub world: World,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            world: World::new(),
        }
    }
    pub fn load_gltf(&mut self, device: &Arc<Device>) {
        let path = "./src/res/cube_scene.gltf";
        let (gltf, buffers, _) = gltf::import(path).unwrap();
        {
            // Texture loading
            let mut texture_entities = HashMap::new();
            for texture in gltf.textures() {
                let image = match texture.source().source() {
                    gltf::image::Source::Uri { uri, mime_type } => {
                        let parent = Path::new(path).parent().unwrap();
                        let image_path = parent.join(uri);
                        let img = image::io::Reader::open(image_path)
                            .unwrap()
                            .decode()
                            .unwrap()
                            .into_rgba8();
                        image::DynamicImage::ImageRgba8(img)
                    }
                    _ => panic!("not supported"),
                };
                let entity = self.world.spawn().insert(Texture(image)).id();
                texture_entities.insert(texture.index(), entity);
            }
            // Mesh loading
            let mut mesh_entities = HashMap::new();
            for mesh in gltf.meshes() {
                let primitive = mesh.primitives().next().unwrap();
                let mut indices: VertexData<Index> = VertexData(Vec::new());
                let mut vertices: Vertices = Vertices {
                    vertices: Vec::new(),
                    has_normal: false,
                    has_uv0: false,
                    has_uv1: false,
                };
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let mut normal_iter = reader.read_normals();
                vertices.has_normal = normal_iter.is_some();
                let mut uv0_iter = reader.read_tex_coords(0).map(|i| i.into_f32());
                vertices.has_uv0 = uv0_iter.is_some();
                let mut uv1_iter = reader.read_tex_coords(0).map(|i| i.into_f32());
                vertices.has_uv1 = uv1_iter.is_some();
                for pos in reader.read_positions().unwrap() {
                    let normal = normal_iter.as_mut().unwrap().next().unwrap_or([0., 0., 0.]);
                    let mut uv0 = [0., 0.];
                    let mut uv1 = [0., 0.];
                    if let Some(uv_iter) = uv0_iter.as_mut() {
                        uv0 = uv_iter.next().unwrap_or([0., 0.]);
                    }
                    if let Some(uv_iter) = uv1_iter.as_mut() {
                        uv1 = uv_iter.next().unwrap_or([0., 0.]);
                    }
                    vertices.vertices.push(Vertex {
                        pos: [pos[0], pos[1], pos[2], 1.],
                        normal: [normal[0], normal[1], normal[2], 0.],
                        uv01: [uv0[0], uv0[1], uv1[0], uv1[0]],
                    });
                }

                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        indices.0.push(Index(index));
                    }
                }
                let mut entity = self.world.spawn();
                entity.insert(indices).insert(vertices);

                let entity = entity.id();
                mesh_entities.insert(mesh.index(), entity);
            }
            // Material loading
            let mut material_entities = HashMap::new();
            for material in gltf.materials() {
                let mr = material.pbr_metallic_roughness();
                let emission = material.emissive_factor();
                let albedo_tex = material
                    .pbr_metallic_roughness()
                    .base_color_texture()
                    .map(|b| TextureId {
                        texture: texture_entities[&b.texture().index()],
                        coords: b.tex_coord(),
                    });
                let mr_tex = material
                    .pbr_metallic_roughness()
                    .metallic_roughness_texture()
                    .map(|b| TextureId {
                        texture: texture_entities[&b.texture().index()],
                        coords: b.tex_coord(),
                    });
                let emission_tex = material.emissive_texture().map(|b| TextureId {
                    texture: texture_entities[&b.texture().index()],
                    coords: b.tex_coord(),
                });
                let normal_tex = material.normal_texture().map(|b| TextureId {
                    texture: texture_entities[&b.texture().index()],
                    coords: b.tex_coord(),
                });
                let material_entity = self
                    .world
                    .spawn()
                    .insert(Material {
                        albedo: mr.base_color_factor(),
                        mr: [mr.metallic_factor(), mr.roughness_factor(), 0., 0.],
                        emission,
                        albedo_tex,
                        mr_tex,
                        emission_tex,
                        normal_tex,
                    })
                    .id();
                material_entities.insert(material.index().unwrap(), material_entity);
            }
            // Instance/Node and Camera loading
            for node in gltf.nodes() {
                if let Some(camera) = node.camera() {
                    if let gltf::camera::Projection::Perspective(proj) = camera.projection() {
                        let transform = Mat4::from_cols_array_2d(&node.transform().matrix());
                        let rot = Mat3::from_mat4(transform);
                        // Not quite sure about the default vectors.
                        let up = rot * Vec3::new(1., 0., 0.);
                        let right = rot * Vec3::new(0., -1., 0.);
                        let pos = transform * Vec4::new(0., 0., 0., 1.);

                        let camera = Camera {
                            up: up.to_array(),
                            right: right.to_array(),
                            pos: pos.xyz().to_array(),
                            focus: 1.,
                            diameter: 0.1,
                            fov: proj.yfov(),
                        };
                        trace!("Camera: {:#?}", camera);

                        //self.world.insert_resource(camera);
                        self.world.spawn().insert(camera);
                    }
                }
                if let Some(mesh) = node.mesh() {
                    let matrix = node.transform().matrix();
                    self.world.spawn().insert_bundle(InstanceBundle {
                        mesh: MeshId(mesh_entities[&mesh.index()]),
                        material: MaterialId(
                            material_entities[&mesh
                                .primitives()
                                .next()
                                .unwrap()
                                .material()
                                .index()
                                .unwrap()],
                        ),
                        transform: Transform::from_matrix(Mat4::from_cols_array_2d(&matrix)),
                    });
                }
            }
        }
    }
}
