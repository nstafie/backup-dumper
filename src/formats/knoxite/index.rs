use std::{collections::HashMap, path::Path};

use serde::Deserialize;

use super::{config::Config, decoder::Decoder};
use crate::error::Result;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub chunks: HashMap<String, IndexItem>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct IndexItem {
    pub hash: String,
    pub data_parts: u32,
    pub parity_parts: Option<u32>,
    pub size: i32,
    pub snapshots: Vec<String>,
}

impl Index {
    pub fn from_file(config: &Config, path: impl AsRef<Path>) -> Result<Index> {
        let file = std::fs::read(path)?;

        let data = Decoder::new_key(&config.key).decrypt_and_decompress(&file)?;

        trace!("Index JSON: {}", String::from_utf8_lossy(&data));
        let index = serde_json::from_slice(&data)?;

        Ok(index)
    }
}
