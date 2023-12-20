use miette::Diagnostic;
use std::string::FromUtf8Error;
use tf_asset_loader::LoaderError;
use thiserror::Error;
use vmt_parser::VdfError;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    Bsp(#[from] vbsp::BspError),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Vtf(#[from] vtf::Error),
    #[error(transparent)]
    Vdf(#[from] VdfError),
    #[error(transparent)]
    Mdl(#[from] vmdl::ModelError),
    #[error("{0}")]
    Other(String),
    #[error(transparent)]
    String(#[from] FromUtf8Error),
    #[error(transparent)]
    Loader(#[from] LoaderError),
    #[error(transparent)]
    Gltf(#[from] gltf::Error),
    #[error("resource {0} not found in vpks or pack")]
    ResourceNotFound(String),
}
