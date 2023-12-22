use gltf_json as json;
mod bsp;
mod error;
pub mod gltf_builder;
mod materials;
mod prop;

use crate::bsp::{bsp_models, push_bsp_model};
use crate::prop::push_or_get_model;
use cgmath::Matrix4;
use clap::Parser;
pub use error::Error;
use gltf::Glb;
use gltf_json::validation::USize64;
use gltf_json::{Buffer, Index, Node, Root, Scene};
use miette::Context;
use std::borrow::Cow;
use std::fs::{read, File};
use std::path::PathBuf;
use tf_asset_loader::Loader;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;
use vbsp::Bsp;

fn setup() {
    miette::set_panic_hook();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .init();
}

/// View a demo file
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path of the map file
    source: PathBuf,
    /// Path to save the glb to
    target: PathBuf,
}

fn main() -> miette::Result<()> {
    setup();

    let args = Args::parse();

    let mut loader = Loader::new().map_err(Error::from)?;
    let data = read(args.source).map_err(Error::from)?;
    let map = Bsp::read(&data).map_err(Error::from)?;
    loader.add_source(map.pack.clone().into_zip());

    let glb = export(map, &loader)?;

    let writer = File::create(&args.target)
        .map_err(Error::from)
        .wrap_err("Failed to open target")?;

    glb.to_writer(writer)
        .map_err(Error::from)
        .wrap_err("glTF binary output error")?;

    Ok(())
}

fn export(bsp: Bsp, loader: &Loader) -> Result<Glb<'static>, Error> {
    let mut buffer = Vec::new();

    let mut root = Root::default();

    for (model, offset) in bsp_models(&bsp)? {
        let node = push_bsp_model(&mut buffer, &mut root, loader, &model, offset);
        root.nodes.push(node);
    }

    for prop in bsp.static_props() {
        let mesh = push_or_get_model(&mut buffer, &mut root, loader, prop.model(), prop.skin);

        let matrix = Matrix4::from_translation(map_coords(prop.origin).into())
            * Matrix4::from(prop.rotation());

        let node = Node {
            camera: None,
            children: None,
            extensions: Default::default(),
            extras: Default::default(),
            matrix: Some([
                matrix.x.x, matrix.x.y, matrix.x.z, matrix.x.w, matrix.y.x, matrix.y.y, matrix.y.z,
                matrix.y.w, matrix.z.x, matrix.z.y, matrix.z.z, matrix.z.w, matrix.w.x, matrix.w.y,
                matrix.w.z, matrix.w.w,
            ]),
            mesh: Some(mesh),
            name: None,
            rotation: None,
            scale: None,
            translation: None,
            skin: None,
            weights: None,
        };
        root.nodes.push(node);
    }

    let node_indices = 0..root.nodes.len();
    root.scenes = vec![Scene {
        name: None,
        extensions: None,
        extras: Default::default(),
        nodes: node_indices.map(|index| Index::new(index as u32)).collect(),
    }];

    root.buffers.push(Buffer {
        byte_length: USize64(buffer.len() as u64),
        extensions: Default::default(),
        extras: Default::default(),
        name: None,
        uri: None,
    });

    let json_string = json::serialize::to_string(&root).expect("Serialization error");
    let mut json_offset = json_string.len() as u32;
    align_to_multiple_of_four(&mut json_offset);

    pad_byte_vector(&mut buffer);
    Ok(Glb {
        header: gltf::binary::Header {
            magic: *b"glTF",
            version: 2,
            length: json_offset + buffer.len() as u32,
        },
        bin: Some(Cow::Owned(buffer)),
        json: Cow::Owned(json_string.into_bytes()),
    })
}

fn align_to_multiple_of_four(n: &mut u32) {
    *n = (*n + 3) & !3;
}

fn pad_byte_vector(vec: &mut Vec<u8>) {
    while vec.len() % 4 != 0 {
        vec.push(0); // pad to multiple of four bytes
    }
}

// 1 hammer unit is ~1.905cm
pub const UNIT_SCALE: f32 = 1.0 / (1.905 * 100.0);

pub fn map_coords<C: Into<[f32; 3]>>(vec: C) -> [f32; 3] {
    let vec = vec.into();
    [
        vec[1] * UNIT_SCALE,
        vec[2] * UNIT_SCALE,
        vec[0] * UNIT_SCALE,
    ]
}
