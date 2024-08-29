mod pack;

use crate::pack::pack;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use clap::Parser;
use http::Method;
use reqwest::Client;
use serde::Deserialize;
use std::fs::{read, read_to_string, write};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Arc;
use tf_asset_loader::{Loader, LoaderError};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::signal;
use toml::from_str;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;
use url::Url;
use vbsp::{Bsp, BspError};
use vbsp_to_gltf::{export, ConvertOptions, Error};

type Result<T, E = ServerError> = std::result::Result<T, E>;

#[derive(Debug, Deserialize)]
struct Config {
    cache_dir: PathBuf,
    map_server: Url,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_port() -> u16 {
    3030
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    Convert(#[from] vbsp_to_gltf::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("IO error {err:#} on {}", path.display())]
    File { err: std::io::Error, path: PathBuf },
    #[error("Error {err:#} while packing {} to {} with {}", input.display(), output.display(), binary)]
    Pack {
        err: std::io::Error,
        input: PathBuf,
        output: PathBuf,
        binary: String,
    },
    #[error(transparent)]
    Loader(#[from] LoaderError),
    #[error(transparent)]
    Req(#[from] reqwest::Error),
    #[error("invalid map name {0}")]
    InvalidMapName(String),
    #[error("failed to optimize output: {0}")]
    GltfPack(String),
    #[error(transparent)]
    TmpFile(#[from] async_tempfile::Error),
}

impl ServerError {
    fn status_code(&self) -> StatusCode {
        match self {
            ServerError::InvalidMapName(_) => StatusCode::UNPROCESSABLE_ENTITY,
            // unexpected header means the dl wasn't a map
            ServerError::Convert(Error::Bsp(BspError::UnexpectedHeader(_))) => {
                StatusCode::NOT_FOUND
            }
            ServerError::Req(e)
                if e.status()
                    .map(|status| status.is_client_error())
                    .unwrap_or_default() =>
            {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        error!(error = ?self, "error during request");
        (self.status_code(), self.to_string()).into_response()
    }
}

/// View a demo file
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Config url
    config: PathBuf,
}

fn setup() -> Result<Config> {
    miette::set_panic_hook();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(
            HierarchicalLayer::new(2)
                .with_targets(true)
                .with_bracketed_fields(true),
        )
        .init();

    let args = Args::parse();
    let toml = read_to_string(&args.config).map_err(|err| ServerError::File {
        path: args.config.into(),
        err,
    })?;
    Ok(from_str(&toml)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = setup()?;

    let app = App {
        cache_dir: config.cache_dir,
        map_server: config.map_server,
        client: Client::default(),
        loader: Loader::new()?,
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let app = Router::new()
        .route("/gltf/:map", get(convert))
        .route("/", get(index))
        .layer(cors)
        .with_state(Arc::new(app));

    // Run our app with hyper
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.port)).await?;
    info!("listening on {}", listener.local_addr()?);
    let serve = async {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("{e:#?}");
        }
    };
    let shutdown = shutdown_signal();
    tokio::select! {
        _ = serve => {},
        _ = shutdown => {},
    }

    Ok(())
}

struct App {
    cache_dir: PathBuf,
    map_server: Url,
    client: Client,
    loader: Loader,
}

impl App {
    fn cache_path(&self, map: &str, options_key: u64) -> PathBuf {
        self.cache_dir.join(format!("{options_key:016x}_{map}"))
    }

    fn cached(&self, map: &str, options_key: u64) -> Result<Option<Vec<u8>>> {
        let path = self.cache_path(map, options_key);
        if path.exists() {
            Ok(Some(read(&path).map_err(|err| ServerError::File {
                path: path.clone(),
                err,
            })?))
        } else {
            Ok(None)
        }
    }

    fn cache(&self, map: &str, data: &[u8], options_key: u64) -> Result<()> {
        let path = self.cache_path(map, options_key);
        Ok(write(&path, data).map_err(|err| ServerError::File {
            path: path.clone(),
            err,
        })?)
    }

    async fn download(&self, map: &str) -> Result<Vec<u8>> {
        info!(map = map, "downloading map");
        Ok(self
            .client
            .get(
                self.map_server
                    .join(map)
                    .map_err(|_| ServerError::InvalidMapName(map.into()))?,
            )
            .send()
            .await?
            .bytes()
            .await?
            .to_vec())
    }
}

async fn index() -> impl IntoResponse {
    Html(include_str!("./index.html"))
}

async fn convert(
    State(app): State<Arc<App>>,
    Path(map): Path<String>,
    Query(mut options): Query<ConvertOptions>,
) -> impl IntoResponse {
    if options.texture_scale > 1.0 {
        options.texture_scale = 1.0;
    }
    let options_key = options.key();
    if !map.is_ascii() || map.contains('/') || !map.ends_with(".glb") {
        return Err(ServerError::InvalidMapName(map));
    }
    if let Some(cached) = app.cached(&map, options_key)? {
        info!(map = map, "serving cached model");
        return Ok(cached);
    }

    let bsp_name = format!("{}.bsp", map.strip_suffix(".glb").unwrap());
    let bsp_data = app.download(&bsp_name).await?;

    let mut loader = app.loader.clone();

    let bsp = Bsp::read(&bsp_data).map_err(Error::from)?;
    loader.add_source(bsp.pack.clone().into_zip());

    let glb = export(bsp, &loader, options)?;
    let glb = glb.to_vec().map_err(Error::from)?;
    let packed = pack(&map, &glb).await?;

    info!(
        unoptimized = glb.len(),
        optimized = packed.len(),
        map = map,
        "optimized model"
    );

    app.cache(&map, &packed, options_key)?;

    Ok(packed)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
