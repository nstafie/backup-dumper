use std::path::Path;

use chrono::prelude::*;
use serde::Deserialize;

use super::{decoder::Decoder, keys::Masterkey};
use crate::{error::Result, utils::from_datetime};

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    #[serde(deserialize_with = "from_datetime")]
    pub time: DateTime<FixedOffset>,
    pub tree: String,
    pub paths: Vec<String>,
    pub hostname: String,
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub tags: Option<Vec<String>>,
    pub original: Option<String>,
}

impl Snapshot {
    pub fn from_file(masterkey: &Masterkey, path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::read(path.as_ref())?;

        let decoder = Decoder::new(masterkey);
        let decoded = decoder.decrypt_and_decompress(&file)?;

        trace!("Snapshot JSON: {}", String::from_utf8_lossy(&decoded));
        let snapshot: Snapshot = serde_json::from_slice(&decoded)?;

        Ok(snapshot)
    }
}
