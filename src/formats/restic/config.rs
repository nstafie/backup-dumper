use std::{self, path::Path};

use serde::Deserialize;

use super::{decoder::Decoder, keys::Masterkey};
use crate::error::Result;

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub id: String,
    pub chunker_polynomial: String,
}

impl Config {
    pub fn from_file(masterkey: &Masterkey, path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::read(path)?;

        let decoder = Decoder::new(masterkey);
        let decoded = decoder.decrypt(&file)?;

        trace!("Config JSON: {}", String::from_utf8_lossy(&decoded));
        let config = serde_json::from_slice(&decoded)?;

        Ok(config)
    }
}
