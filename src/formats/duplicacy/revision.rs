use std::path::Path;

use serde::Deserialize;

use super::{config::Config, decoder::Decoder};
use crate::{error::Result, utils::from_hex_vec};

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub version: i32,
    pub id: String,
    pub revision: i32,
    pub options: String,
    pub tag: String,
    pub start_time: i64,
    pub end_time: i64,
    pub file_size: i64,
    pub number_of_files: i64,

    #[serde(deserialize_with = "from_hex_vec")]
    pub files: Vec<Vec<u8>>,
    #[serde(deserialize_with = "from_hex_vec")]
    pub chunks: Vec<Vec<u8>>,
    #[serde(deserialize_with = "from_hex_vec")]
    pub lengths: Vec<Vec<u8>>,
}

impl Revision {
    pub fn from_file(config: &Config, path: impl AsRef<Path>) -> Result<Self> {
        // read into memory
        let file = std::fs::read(path.as_ref())?;

        let decoded = match config.encrypted {
            true => {
                let components = path
                    .as_ref()
                    .iter()
                    .map(|c| c.to_str().unwrap())
                    .collect::<Vec<_>>();
                let derivation = components[components.len() - 3..].join("/");
                trace!("Derivation: {}", derivation);

                let key = config.derive_key(&config.file_key, derivation.as_bytes())?;
                let decoder = Decoder::new(key);
                decoder.decode(&file)?
            }
            false => {
                let decoder = Decoder::new(None);
                decoder.decode(&file)?
            }
        };

        trace!("Revision JSON: {}", String::from_utf8_lossy(&decoded));
        let revision: Self = serde_json::from_slice(&decoded)?;

        Ok(revision)
    }
}
