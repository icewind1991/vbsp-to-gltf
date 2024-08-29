use crate::{ConvertOptions, Error};
use image::imageops::FilterType;
use image::DynamicImage;
use tf_asset_loader::Loader;
use tracing::{error, instrument};
use vmt_parser::material::{Material, WaterMaterial};
use vmt_parser::{from_str, TextureTransform};
use vtf::vtf::VTF;

pub fn load_material_fallback(
    name: &str,
    search_dirs: &[String],
    loader: &Loader,
    options: &ConvertOptions,
) -> MaterialData {
    match load_material(name, search_dirs, loader, options) {
        Ok(mat) => mat,
        Err(e) => {
            error!(error = ?e, "failed to load material");
            MaterialData {
                name: name.into(),
                color: [255, 0, 255, 255],
                ..MaterialData::default()
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct MaterialData {
    pub name: String,
    pub path: String,
    pub color: [u8; 4],
    pub texture: Option<TextureData>,
    pub alpha_test: Option<f32>,
    pub bump_map: Option<TextureData>,
    pub translucent: bool,
    pub no_cull: bool,
    pub transform: Option<TextureTransform>,
}

#[derive(Debug)]
pub struct TextureData {
    pub name: String,
    pub image: DynamicImage,
}

#[instrument(skip(loader))]
pub fn load_material(
    name: &str,
    search_dirs: &[String],
    loader: &Loader,
    options: &ConvertOptions,
) -> Result<MaterialData, Error> {
    let dirs = search_dirs
        .iter()
        .map(|dir| {
            format!(
                "materials/{}",
                dir.to_ascii_lowercase().trim_start_matches('/')
            )
        })
        .collect::<Vec<_>>();
    let path = format!("{}.vmt", name.to_ascii_lowercase().trim_end_matches(".vmt"));
    let path = loader
        .find_in_paths(&path, &dirs)
        .ok_or(Error::Other(format!("Can't find file {}", path)))?;
    let raw = loader.load(&path)?.expect("didn't find foudn path?");
    let vdf = String::from_utf8(raw)?;

    let material = from_str(&vdf).map_err(|e| {
        let report = miette::ErrReport::new(e);
        println!("{:?}", report);
        Error::Other(format!("Failed to load material {}", path))
    })?;
    let material = material.resolve(|path| {
        let data = loader
            .load(path)?
            .ok_or(Error::Other(format!("Can't find file {}", path)))?;
        let vdf = String::from_utf8(data)?;
        Ok::<_, Error>(vdf)
    })?;

    if let Material::Water(WaterMaterial {
        base_texture: None, ..
    }) = &material
    {
        return Ok(MaterialData {
            color: [82, 180, 217, 128],
            name: name.into(),
            path,
            translucent: true,
            ..MaterialData::default()
        });
    }

    let base_texture = material
        .base_texture()
        .ok_or_else(|| Error::Other("no basetexture".into()))?;

    let translucent = material.translucent();
    let glass = material.surface_prop() == Some("glass");
    let alpha_test = material.alpha_test();
    let texture = load_texture(base_texture, loader, options)?;

    let bump_map = material.bump_map().and_then(|path| {
        Some(TextureData {
            image: load_texture(path, loader, options).ok()?,
            name: path.into(),
        })
    });

    let transform = material
        .base_texture_transform()
        .filter(|transform| **transform != TextureTransform::default())
        .cloned();

    Ok(MaterialData {
        color: [255; 4],
        name: name.into(),
        path,
        texture: Some(TextureData {
            name: base_texture.into(),
            image: texture,
        }),
        bump_map,
        alpha_test,
        translucent: translucent | glass,
        no_cull: material.no_cull(),
        transform,
    })
}

fn load_texture(
    name: &str,
    loader: &Loader,
    options: &ConvertOptions,
) -> Result<DynamicImage, Error> {
    let path = format!(
        "materials/{}.vtf",
        name.trim_end_matches(".vtf").trim_start_matches('/')
    );
    let raw = loader
        .load(&path)?
        .ok_or(Error::Other(format!("Can't find file {}", path)))?;
    let vtf = VTF::read(&raw)?;
    let image = vtf.highres_image.decode(0)?;
    if options.texture_scale != 1.0 {
        Ok(image.resize(
            (image.width() as f32 * options.texture_scale) as u32,
            (image.height() as f32 * options.texture_scale) as u32,
            FilterType::CatmullRom,
        ))
    } else {
        Ok(image)
    }
}
