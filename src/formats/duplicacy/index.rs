use std::path::Path;

use serde::Deserialize;

use super::{config::Config, decoder::Decoder};
use crate::{error::Result, utils::from_hex_vec};

#[derive(Deserialize, Debug)]
#[serde(transparent)]
pub struct Index {
    #[serde(deserialize_with = "from_hex_vec")]
    pub hashes: Vec<Vec<u8>>,
}

impl Index {
    pub fn from_file(config: &Config, path: impl AsRef<Path>, hash: &[u8]) -> Result<Self> {
        let file = std::fs::read(path.as_ref())?;

        let decoded = if config.encrypted {
            let key = config.derive_key(&config.chunk_key, hash)?;
            let decoder = Decoder::new(key);
            decoder.decode(&file)?
        } else {
            let decoder = Decoder::new(None);
            decoder.decode(&file)?
        };

        // parse the entries
        //trace!("Index JSON: {}", String::from_utf8_lossy(&decoded));
        let chunk: Self = serde_json::from_slice(&decoded)?;

        Ok(chunk)
    }
}
