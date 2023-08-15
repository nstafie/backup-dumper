use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Deserializer};

use super::{decoder::Decoder, keys::Keys};
use crate::error::Result;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub data_format_version: u32,
    pub snapshot: HashMap<String, Item>,
    pub chunks: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Item {
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub mtime: f64,
    #[serde(default, deserialize_with = "from_array")]
    pub range: Option<ChunkRange>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum ItemType {
    Dir,
    File,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkRange {
    pub start_chunk: usize,
    pub start_offset: usize,
    pub end_chunk: usize,
    pub end_offset: usize,
}

impl Snapshot {
    pub fn from_file(keys: &Keys, path: impl AsRef<Path>) -> Result<Snapshot> {
        let file = std::fs::read(path)?;

        let decoder = Decoder::new(&keys.master_key);
        let data = decoder.decrypt_and_decompress(&file)?;

        trace!("Snapshot JSON: {}", String::from_utf8_lossy(&data));
        let snapshot = serde_json::from_slice(&data)?;

        Ok(snapshot)
    }
}

pub fn from_array<'de, D: Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<ChunkRange>, D::Error> {
    let opt = Option::<[usize; 4]>::deserialize(d)?;
    if let Some(array) = opt {
        Ok(Some(ChunkRange {
            start_chunk: array[0],
            start_offset: array[1],
            end_chunk: array[2],
            end_offset: array[3],
        }))
    } else {
        Ok(None)
    }
}
