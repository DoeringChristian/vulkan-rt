use crate::model::{
    GlslCamera, Index, InstanceKey, Light, Material, Medium, MeshInstance, ShaderGroupKey, Vertex,
};
use glam::*;
use screen_13::prelude::Device;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::renderer::RTRenderer;

use super::Loader;

#[derive(Default)]
pub struct GltfLoader {}

impl Loader<Mutex<RTRenderer>> for GltfLoader {
    type Ctx = Arc<Device>;

    fn load_to(
        &self,
        path: impl AsRef<std::path::Path>,
        ctx: &Self::Ctx,
        dst: &Mutex<RTRenderer>,
        default_hit_groups: Vec<ShaderGroupKey>,
    ) -> Vec<InstanceKey> {
        let path = path.as_ref();
        let mut instances = vec![];
        let (gltf, buffers, _) = gltf::import(path).unwrap();

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
                _ => unimplemented!(),
            };
            let entity = dst.lock().unwrap().insert_texture(ctx, &image);
            texture_entities.insert(texture.index(), entity);
        }
        // Mesh loading
        let mut mesh_entities = HashMap::new();
        for mesh in gltf.meshes() {
            let primitive = mesh.primitives().next().unwrap();
            let mut indices = vec![];
            let mut vertices = vec![];
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let mut normal_iter = reader.read_normals();
            let mut uv0_iter = reader.read_tex_coords(0).map(|i| i.into_f32());
            let mut uv1_iter = reader.read_tex_coords(1).map(|i| i.into_f32());
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
                vertices.push(Vertex {
                    pos: [pos[0], pos[1], pos[2], 1.],
                    normal: [normal[0], normal[1], normal[2], 0.],
                    uv01: [uv0[0], uv0[1], uv1[0], uv1[1]],
                });
            }
            if let Some(iter) = reader.read_indices() {
                for index in iter.into_u32() {
                    indices.push(Index(index));
                }
            }
            let entity = dst.lock().unwrap().insert_mesh(ctx, &indices, &vertices);
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
                .map(|b| texture_entities[&b.texture().index()]);
            let mr_tex = material
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
                .map(|b| texture_entities[&b.texture().index()]);
            let emission_tex = material
                .emissive_texture()
                .map(|b| texture_entities[&b.texture().index()]);
            let normal_tex = material
                .normal_texture()
                .map(|b| texture_entities[&b.texture().index()]);
            let transmission = material
                .transmission()
                .map(|t| t.transmission_factor())
                .unwrap_or(0.);
            let transmission_tex = material
                .transmission()
                .map(|t| {
                    t.transmission_texture()
                        .map(|t| texture_entities[&t.texture().index()])
                })
                .flatten();
            let ior = material.ior().unwrap_or(1.4);
            let material_entity = dst.lock().unwrap().insert_material(Material {
                albedo: Vec4::from(mr.base_color_factor()),
                metallic: mr.metallic_factor(),
                roughness: mr.roughness_factor(),
                emission: Vec3::from(emission),
                transmission,
                transmission_roughness: 0.,
                ior,
                albedo_tex,
                mr_tex,
                emission_tex,
                normal_tex,
                transmission_tex,
                medium: Medium {
                    color: Vec4::from(mr.base_color_factor()),
                    anisotropic: 0.,
                    density: 1. - transmission,
                },
            });
            material_entities.insert(material.index().unwrap(), material_entity);
        }
        // Loading Nodes: Instances, Cameras, Lights
        for node in gltf.nodes() {
            if let Some(camera) = node.camera() {
                if let gltf::camera::Projection::Perspective(proj) = camera.projection() {
                    let transform = Mat4::from_cols_array_2d(&node.transform().matrix());
                    let rot = Mat3::from_mat4(transform);
                    let up = rot * Vec3::new(0., 1., 0.);
                    let right = rot * Vec3::new(1., 0., 0.);
                    let pos = transform * Vec4::new(0., 0., 0., 1.);

                    let up = up.to_array();
                    let right = right.to_array();
                    let pos = pos.to_array();
                    dst.lock().unwrap().set_camera(GlslCamera {
                        up: [up[0], up[1], up[2], 1.],
                        right: [right[0], right[1], right[2], 1.],
                        pos: [pos[0], pos[1], pos[2], 1.],
                        focus: 1.,
                        diameter: 0.1,
                        fov: proj.yfov(),
                        fc: 0,
                        depth: 16,
                    });
                }
            }
            if let Some(mesh) = node.mesh() {
                let matrix = node.transform().matrix();
                instances.push(
                    dst.lock().unwrap().insert_instance(MeshInstance {
                        transform: Mat4::from_cols_array_2d(&matrix),
                        material: material_entities[&mesh
                            .primitives()
                            .next()
                            .unwrap()
                            .material()
                            .index()
                            .unwrap()],
                        mesh: mesh_entities[&mesh.index()],
                        shader_groups: default_hit_groups.clone(),
                    }),
                );
            }
            if let Some(light) = node.light() {
                let transform = node.transform().matrix();
                let pos = Mat4::from_cols_array_2d(&transform) * vec4(0., 0., 0., 1.);
                dst.lock().unwrap().insert_light(Light::Point {
                    emission: Vec3::from(light.color()),
                    position: pos.xyz(),
                    radius: 0.2,
                    strength: light.intensity(),
                });
                println!("intensity: {}", light.intensity());
            }
        }
        instances
    }
}
