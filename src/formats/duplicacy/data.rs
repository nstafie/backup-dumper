use std::path::Path;

use serde::Deserialize;

use super::{config::Config, decoder::Decoder};
use crate::error::Result;

#[derive(Deserialize, Debug)]
pub struct Data {
    pub data: Vec<u8>,
}

impl Data {
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

        // this chunk type contains ONLY raw data
        Ok(Data { data: decoded })
    }
}
