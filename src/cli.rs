use clap::Parser;
use miette::Context;
use std::fs::{read, File};
use std::path::PathBuf;
use tf_asset_loader::Loader;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;
use vbsp::Bsp;
use vbsp_to_gltf::{export, ConvertOptions, Error};

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

    let glb = export(map, &loader, ConvertOptions::default())?;

    let writer = File::create(&args.target)
        .map_err(Error::from)
        .wrap_err("Failed to open target")?;

    glb.to_writer(writer)
        .map_err(Error::from)
        .wrap_err("glTF binary output error")?;

    Ok(())
}
