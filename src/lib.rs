mod bsp;
pub mod convert;
mod error;
pub mod gltf_builder;
mod materials;
mod prop;

use ahash::RandomState;
pub use convert::export;
pub use error::Error;
use serde::Deserialize;
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Debug, Deserialize, Clone)]
pub struct ConvertOptions {
    #[serde(default = "default_enable")]
    pub textures: bool,
    #[serde(default = "default_scale")]
    pub texture_scale: f32,
}

impl ConvertOptions {
    pub fn key(&self) -> u64 {
        let mut hasher = RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        self.textures.hash(&mut hasher);
        self.texture_scale.to_le_bytes().hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for ConvertOptions {
    fn default() -> Self {
        ConvertOptions {
            textures: true,
            texture_scale: 1.0,
        }
    }
}

fn default_enable() -> bool {
    true
}

fn default_scale() -> f32 {
    1.0
}
