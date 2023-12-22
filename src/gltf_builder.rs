use crate::materials::{load_material_fallback, MaterialData, TextureData};
use crate::pad_byte_vector;
use gltf_json::buffer::View;
use gltf_json::image::MimeType;
use gltf_json::material::{AlphaCutoff, AlphaMode, PbrBaseColorFactor, PbrMetallicRoughness};
use gltf_json::texture::Info;
use gltf_json::validation::Checked::Valid;
use gltf_json::validation::USize64;
use gltf_json::{Extras, Image, Index, Material, Root, Texture};
use image::png::PngEncoder;
use image::{ColorType, DynamicImage, GenericImageView};
use tf_asset_loader::Loader;

pub fn push_or_get_material(
    buffer: &mut Vec<u8>,
    gltf: &mut Root,
    loader: &Loader,
    material: &str,
) -> Index<Material> {
    let material = material.to_ascii_lowercase();
    match get_material_index(&gltf.materials, &material) {
        Some(index) => index,
        None => {
            let material = load_material_fallback(&material, &[String::new()], loader);
            let index = gltf.materials.len() as u32;
            let material = push_material(buffer, gltf, material);
            gltf.materials.push(material);
            Index::new(index)
        }
    }
}

fn get_material_index(materials: &[Material], path: &str) -> Option<Index<Material>> {
    materials
        .iter()
        .enumerate()
        .find_map(|(i, mat)| (mat.name.as_deref() == Some(path)).then_some(i))
        .map(|i| Index::new(i as u32))
}

pub fn push_material(buffer: &mut Vec<u8>, gltf: &mut Root, material: MaterialData) -> Material {
    let texture_index = material
        .texture
        .map(|tex| push_or_get_texture(buffer, gltf, tex));

    let alpha_mode = match (material.translucent, material.alpha_test.is_some()) {
        (true, _) => AlphaMode::Blend,
        (false, true) => AlphaMode::Mask,
        _ => AlphaMode::Opaque,
    };

    Material {
        name: Some(material.name),
        alpha_cutoff: material
            .alpha_test
            .map(AlphaCutoff)
            .filter(|_| alpha_mode == AlphaMode::Mask),
        double_sided: material.no_cull,
        alpha_mode: Valid(alpha_mode),
        pbr_metallic_roughness: PbrMetallicRoughness {
            base_color_factor: PbrBaseColorFactor(
                material.color.map(|channel| channel as f32 / 255.0),
            ),
            base_color_texture: texture_index.map(|index| Info {
                index,
                tex_coord: 0,
                extensions: None,
                extras: Extras::default(),
            }),
            ..PbrMetallicRoughness::default()
        },
        ..Material::default()
    }
}

fn push_or_get_texture(
    buffer: &mut Vec<u8>,
    gltf: &mut Root,
    texture: TextureData,
) -> Index<Texture> {
    match get_texture_index(&gltf.textures, &texture.name) {
        Some(index) => index,
        None => {
            let index = gltf.textures.len() as u32;
            let texture = push_texture(buffer, gltf, texture);
            gltf.textures.push(texture);
            Index::new(index)
        }
    }
}

fn get_texture_index(textures: &[Texture], name: &str) -> Option<Index<Texture>> {
    textures
        .iter()
        .enumerate()
        .find_map(|(i, tex)| (tex.name.as_deref() == Some(name)).then_some(i))
        .map(|i| Index::new(i as u32))
}

fn push_texture(buffer: &mut Vec<u8>, gltf: &mut Root, texture: TextureData) -> Texture {
    let mut image = texture.image;
    if image.color() != ColorType::Rgba8 && image.color() != ColorType::Rgb8 {
        if image.color().has_alpha() {
            image = DynamicImage::ImageRgba8(image.into_rgba8());
        } else {
            image = DynamicImage::ImageRgb8(image.into_rgb8());
        }
    }
    let buffer_start = buffer.len() as u64;
    let view_start = gltf.buffer_views.len() as u32;
    let image_start = gltf.images.len() as u32;

    let mut png_buffer = Vec::new();
    let encoder = PngEncoder::new(&mut png_buffer);
    encoder
        .encode(
            image.as_bytes(),
            image.width(),
            image.height(),
            image.color(),
        )
        .expect("failed to encode");

    buffer.extend_from_slice(&png_buffer);

    let byte_length = buffer.len() as u64 - buffer_start;
    pad_byte_vector(buffer);

    let view = View {
        buffer: Index::new(0),
        byte_length: USize64(byte_length),
        byte_offset: Some(USize64(buffer_start)),
        byte_stride: None,
        extensions: Default::default(),
        extras: Default::default(),
        name: Some(texture.name.clone()),
        target: None,
    };

    gltf.buffer_views.push(view);

    let image = Image {
        buffer_view: Some(Index::new(view_start)),
        mime_type: Some(MimeType("image/png".into())),
        name: Some(texture.name.clone()),
        uri: None,
        extensions: None,
        extras: Default::default(),
    };
    gltf.images.push(image);

    Texture {
        name: Some(texture.name),
        sampler: None,
        source: Index::new(image_start),
        extensions: None,
        extras: Default::default(),
    }
}
