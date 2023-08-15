use std::path::Path;

use super::{decoder::Decoder, keys::Keys};
use crate::error::Result;

#[derive(Debug)]
pub struct Chunk {
    pub data: Vec<u8>,
}

impl Chunk {
    pub fn from_file(keys: &Keys, path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::read(path)?;

        let decoder = Decoder::new(&keys.master_key);
        let data = decoder.decrypt_and_decompress(&file)?;

        Ok(Self { data })
    }
}
