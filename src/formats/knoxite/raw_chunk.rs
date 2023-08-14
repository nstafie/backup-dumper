use std::path::Path;

use super::{config::Config, decoder::Decoder};
use crate::error::Result;

pub struct RawChunk(pub Vec<u8>);

impl RawChunk {
    pub fn from_file(config: &Config, path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::read(path)?;

        let data = Decoder::new_key(&config.key).decrypt(&file)?;

        Ok(Self(data))
    }
}
