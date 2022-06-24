use crate::accel::{Blas, BlasGeometry, BlasInfo, Tlas};
use crate::buffers::TypedBuffer;
use crate::model::{
    Camera, GlslCamera, GlslInstanceData, GlslMaterial, Index, InstanceBundle, Material,
    MaterialId, MeshId, Normal, Position, Tangent, TexCoord, TexCoords, Texture, TextureId,
    VertexData,
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

#[derive(Debug)]
pub enum UpdateState {
    Updated,
}

#[derive(Component, Debug)]
pub struct GpuResource {
    pub index: usize,
    pub state: UpdateState,
}

pub struct GpuScene {
    //pub world: World,
    //pub geometries: Vec<BlasGeometry>,
    pub blases: Vec<Blas>,
    pub tlas: Tlas,

    pub material_buf: TypedBuffer<GlslMaterial>,
    pub instancedata_buf: TypedBuffer<GlslInstanceData>,
    pub positions_bufs: Vec<Arc<TypedBuffer<Position>>>,
    pub indices_bufs: Vec<Arc<TypedBuffer<Index>>>,
    pub normals_bufs: Vec<Arc<TypedBuffer<Normal>>>,
    pub tex_coords_bufs: Vec<Arc<TypedBuffer<TexCoord>>>,
    pub textures: Vec<Arc<Image>>,
    pub camera: GlslCamera,
}

impl GpuScene {
    pub fn create(device: &Arc<Device>, scene: &mut Scene) -> Self {
        let mut positions_bufs = vec![];
        let mut indices_bufs = vec![];
        let mut normals_bufs = vec![];
        let mut tex_coords_bufs = vec![];
        let mut blases = vec![];
        //let mut mesh_idxs = HashMap::new();
        let mut queue = CommandQueue::from_world(&mut scene.world);
        #[derive(Component, Debug, Clone, Copy)]
        pub struct GpuMesh {
            positions: usize,
            indices: usize,
            normals: Option<usize>,
            tex_coords: Option<(usize, usize)>,
            blas: usize,
        }
        for (entity, positions, indices, normals, tex_coords) in scene
            .world
            .query::<(
                Entity,
                &VertexData<Position>,
                &VertexData<Index>,
                Option<&VertexData<Normal>>,
                Option<&TexCoords>,
            )>()
            .iter(&scene.world)
        {
            let mut mesh_idx = GpuMesh {
                positions: positions_bufs.len(),
                indices: indices_bufs.len(),
                normals: normals.map(|_| normals_bufs.len()),
                tex_coords: tex_coords.map(|_| (tex_coords_bufs.len(), 0)),
                blas: blases.len(),
            };
            //trace!("positions: {}", positions.0.len());
            positions_bufs.push(Arc::new(TypedBuffer::create(
                device,
                &positions.0,
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
            if let Some(normals) = normals {
                normals_bufs.push(Arc::new(TypedBuffer::create(
                    device,
                    &normals.0,
                    vk::BufferUsageFlags::STORAGE_BUFFER,
                )));
            }
            if let Some(tex_coords) = tex_coords {
                for tex_coords in tex_coords.0.iter() {
                    tex_coords_bufs.push(Arc::new(TypedBuffer::create(
                        device,
                        &tex_coords.0,
                        vk::BufferUsageFlags::STORAGE_BUFFER,
                    )));
                    mesh_idx.tex_coords.as_mut().unwrap().1 += 1;
                }
            }
            blases.push(Blas::create(
                device,
                &BlasInfo {
                    indices: indices_bufs.last().unwrap(),
                    positions: positions_bufs.last().unwrap(),
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
        let mut textures_idxs = HashMap::new();
        let mut img_loader = ImageLoader::new(device).unwrap();
        for (entity, texture) in scene.world.query::<(Entity, &Texture)>().iter(&scene.world) {
            textures_idxs.insert(entity, textures.len());
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
        }
        let mut materials = vec![];
        let mut material_idxs = HashMap::new();
        for (entity, material) in scene
            .world
            .query::<(Entity, &Material)>()
            .iter(&scene.world)
        {
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
                    .map(|dt| textures_idxs[&dt.texture])
                    .unwrap_or(INDEX_UNDEF as _) as _,
                diffuse_texco: material
                    .albedo_tex
                    .as_ref()
                    .map(|dt| dt.coords)
                    .unwrap_or(INDEX_UNDEF as _) as _,
                mr_tex: material
                    .mr_tex
                    .as_ref()
                    .map(|dt| textures_idxs[&dt.texture])
                    .unwrap_or(INDEX_UNDEF as _) as _,
                mr_texco: material
                    .mr_tex
                    .as_ref()
                    .map(|dt| dt.coords)
                    .unwrap_or(INDEX_UNDEF as _) as _,
                emission_tex: material
                    .emission_tex
                    .as_ref()
                    .map(|dt| textures_idxs[&dt.texture])
                    .unwrap_or(INDEX_UNDEF as _) as _,
                emission_texco: material
                    .emission_tex
                    .as_ref()
                    .map(|dt| dt.coords)
                    .unwrap_or(INDEX_UNDEF as _) as _,
                normal_tex: material
                    .normal_tex
                    .as_ref()
                    .map(|dt| textures_idxs[&dt.texture])
                    .unwrap_or(INDEX_UNDEF as _) as _,
                normal_texco: material
                    .normal_tex
                    .as_ref()
                    .map(|dt| dt.coords)
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
                .get::<GpuMesh>()
                .unwrap();
            instancedata.push(GlslInstanceData {
                transform: transform.compute_matrix().to_cols_array_2d(),
                mat_index: material_idxs[&material_id.0] as _,
                positions: mesh_idx.positions as _,
                indices: mesh_idx.indices as _,
                normals: mesh_idx.normals.unwrap_or(INDEX_UNDEF as usize) as _,
                tex_coords: mesh_idx
                    .tex_coords
                    .map(|(i, _)| i)
                    .unwrap_or(INDEX_UNDEF as usize) as _,
                tex_coords_num: mesh_idx.tex_coords.map(|(_, n)| n).unwrap_or(0) as _,
                _pad: [0, 0],
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
        let camera = if let Some(camera) = scene.world.get_resource::<Camera>() {
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
            positions_bufs,
            indices_bufs,
            normals_bufs,
            tex_coords_bufs,
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
                let mut positions: VertexData<Position> = VertexData(Vec::new());
                let mut normals: Option<VertexData<Normal>> = None;
                let mut tangents: Option<VertexData<Tangent>> = None;
                let mut tex_coords = TexCoords(Vec::new());
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(iter) = reader.read_positions() {
                    for position in iter {
                        positions.0.push(Position(position));
                    }
                }
                if let Some(iter) = reader.read_indices() {
                    for index in iter.into_u32() {
                        indices.0.push(Index(index));
                    }
                }
                if let Some(iter) = reader.read_normals() {
                    normals = Some(VertexData(Vec::new()));
                    for normal in iter {
                        normals.as_mut().unwrap().0.push(Normal(normal));
                    }
                }
                if let Some(iter) = reader.read_tangents() {
                    tangents = Some(VertexData(Vec::new()));
                    for tangent in iter {
                        tangents.as_mut().unwrap().0.push(Tangent(tangent));
                    }
                }
                while let Some(iter) = reader.read_tex_coords(tex_coords.0.len() as _) {
                    tex_coords.0.push(VertexData(Vec::new()));
                    for tex_coord in iter.into_f32() {
                        tex_coords.0.last_mut().unwrap().0.push(TexCoord(tex_coord));
                    }
                }
                let mut entity = self.world.spawn();
                entity.insert(indices).insert(positions).insert(tex_coords);

                if let Some(normals) = normals {
                    entity.insert(normals);
                }
                if let Some(tangents) = tangents {
                    entity.insert(tangents);
                }

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

                        self.world.insert_resource(camera);
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
