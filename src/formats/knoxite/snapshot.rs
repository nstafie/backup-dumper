use std::{collections::HashMap, path::Path};

use serde::Deserialize;
use serde_repr::Deserialize_repr;

use super::config::Config;
use crate::{error::Result, formats::knoxite::decoder::Decoder, utils::from_truthy};

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub id: String,
    pub date: String,
    pub description: Option<String>,
    pub stats: Stats,
    pub archives: HashMap<String, Archive>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub files: u64,
    pub dirs: u64,
    pub symlinks: u64,
    pub size: u64,
    pub storage_size: u64,
    pub transferred: u64,
    pub errors: u64,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub path: String,
    pub points_to: Option<String>,
    pub mode: u64,
    pub mod_time: i64,
    pub size: u64,
    pub storage_size: u64,
    pub uid: u32,
    pub gid: u32,
    pub chunks: Option<Vec<Chunk>>,
    #[serde(deserialize_with = "from_truthy")]
    pub encrypted: bool,
    #[serde(deserialize_with = "from_truthy")]
    pub compressed: bool,
    #[serde(rename = "type")]
    pub archive_type: ArchiveType,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Chunk {
    pub data: Vec<u8>,
    pub data_parts: u32,
    pub parity_parts: u32,
    pub original_size: i32,
    pub size: i32,
    pub decrypted_hash: String,
    pub hash: String,
    pub num: u32,
}

#[derive(Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
pub enum ArchiveType {
    File = 0,
    Directory = 1,
    Symlink = 2,
}

impl Snapshot {
    pub fn from_file(config: &Config, path: impl AsRef<Path>) -> Result<Snapshot> {
        let file = std::fs::read(path)?;

        let data = Decoder::new_key(&config.key).decrypt_and_decompress(&file)?;

        trace!("Snapshot JSON: {}", String::from_utf8_lossy(&data));
        let snapshot = serde_json::from_slice(&data)?;

        Ok(snapshot)
    }
}
