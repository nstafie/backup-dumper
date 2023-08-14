use std::path::Path;

use serde::Deserialize;

use super::decoder::Decoder;
use crate::error::Result;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub volumes: Vec<Volume>,
    pub paths: Vec<String>,
    pub key: String,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Volume {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub snapshots: Vec<String>,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>, password: impl AsRef<str>) -> Result<Self> {
        let file = std::fs::read(path)?;

        let data = Decoder::new_password(password).decrypt(&file)?;

        trace!("Config JSON: {}", String::from_utf8_lossy(&data));
        let config = serde_json::from_slice(&data)?;

        Ok(config)
    }
}
