use crate::glsl::*;
use glam::*;
use screen_13_fx::ImageLoader;
use std::path::Path;

use crate::scene::Scene;

use super::Loader;

#[derive(Default)]
pub struct GltfLoader {}

impl Loader<Scene> for GltfLoader {
    fn append(&self, path: impl AsRef<Path>, dst: &mut Scene) -> usize {
        let path = path.as_ref();
        let (gltf, buffers, _) = gltf::import(path).unwrap();

        let texture_offset = dst.textures.len();
        for texture in gltf.textures() {
            let img = match texture.source().source() {
                gltf::image::Source::Uri { uri, mime_type } => {
                    let parent = Path::new(path).parent().unwrap();
                    let img_path = parent.join(uri);
                    let img = image::io::Reader::open(img_path)
                        .unwrap()
                        .decode()
                        .unwrap()
                        .into_rgba8();
                    image::DynamicImage::ImageRgba8(img)
                }
                _ => unimplemented!(),
            };
            dst.textures.push(img);
        }

        let mesh_offset = dst.meshes.len();
        for mesh in gltf.meshes() {
            let indices_offset = dst.indices.len();
            let positions_offset = dst.positions.len();
            let normals_offset = dst.normals.len();
            let uvs_offset = dst.uvs.len();

            let primitive = mesh.primitives().next().unwrap();
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            for pos in reader.read_positions().unwrap() {
                dst.positions.push(vec3(pos[0], pos[1], pos[2]));
            }
            for index in reader.read_indices().unwrap().into_u32() {
                dst.indices.push(index);
            }
            for normal in reader.read_normals().unwrap() {
                dst.normals.push(vec3(normal[0], normal[1], normal[2]));
            }
            for uv in reader.read_tex_coords(0).unwrap().into_f32() {
                dst.uvs.push(vec2(uv[0], uv[1]));
            }

            dst.meshes.push(Mesh {
                indices: indices_offset as u32,
                indices_count: dst.indices.len() as u32 - indices_offset as u32,
                positions: positions_offset as u32,
                normals: normals_offset as u32,
                uvs: uvs_offset as u32,
            })
        }

        let material_offset = dst.materials.len();
        for material in gltf.materials() {
            let emission = material.emissive_factor();
            let mr_model = material.pbr_metallic_roughness();

            let base_color = mr_model
                .base_color_texture()
                .map(|t| Texture::image(texture_offset as u32 + t.texture().index() as u32))
                .unwrap_or(Texture::constant(
                    Vec4::from(mr_model.base_color_factor()).xyz(),
                ));
            let metallic_roughness = mr_model
                .metallic_roughness_texture()
                .map(|t| Texture::image(texture_offset as u32 + t.texture().index() as u32))
                .unwrap_or(Texture::constant(vec3(
                    mr_model.metallic_factor(),
                    mr_model.roughness_factor(),
                    0.,
                )));
            let emission = material
                .emissive_texture()
                .map(|t| Texture::image(texture_offset as u32 + t.texture().index() as u32))
                .unwrap_or(Texture::constant(Vec3::from(material.emissive_factor())));
            let normal = material
                .normal_texture()
                .map(|t| Texture::image(texture_offset as u32 + t.texture().index() as u32))
                .unwrap_or(Texture::constant(vec3(0., 0., 1.)));
            let transmission = material
                .transmission()
                .map(|t| {
                    t.transmission_texture()
                        .map(|t| Texture::image(texture_offset as u32 + t.texture().index() as u32))
                        .unwrap_or(Texture::constant(vec3(t.transmission_factor(), 0., 0.)))
                })
                .unwrap_or(Texture::constant(vec3(0., 0., 0.)));

            dst.materials.push(Material {
                base_color,
                metallic_roughness,
                emission,
                normal,
                transmission,
            })
        }

        let instance_offset = dst.instances.len();
        for node in gltf.nodes() {
            if let Some(camera) = node.camera() {
                if let gltf::camera::Projection::Perspective(proj) = camera.projection() {
                    let to_world = Mat4::from_cols_array_2d(&node.transform().matrix());
                    dst.cameras.push(Camera::perspective(
                        to_world,
                        proj.yfov(),
                        proj.aspect_ratio().unwrap_or(1.),
                        0.001,
                        10000.,
                    ));
                }
            }
            if let Some(mesh) = node.mesh() {
                let matrix = node.transform().matrix();
                let mut emitter = -1;
                let material = mesh.primitives().next().unwrap().material();

                if material.emissive_texture().is_some()
                    || material.emissive_factor() != [0., 0., 0.]
                {
                    emitter = dst.emitters.len() as _;
                    let emission = material
                        .emissive_texture()
                        .map(|t| Texture::image(texture_offset as u32 + t.texture().index() as u32))
                        .unwrap_or(Texture::constant(Vec3::from(material.emissive_factor())));
                    dst.emitters.push(Emitter::area(emission, 0));
                }

                let instance = dst.instances.len();
                dst.instances.push(Instance {
                    to_world: Mat4::from_cols_array_2d(&matrix),
                    mesh: mesh_offset as u32 + mesh.index() as u32,
                    material: material_offset as u32 + material.index().unwrap() as u32,
                    emitter,
                });
                if emitter >= 0 {
                    dst.emitters[emitter as usize].instance = instance as u32;
                }
            }
        }
        instance_offset
    }
}
