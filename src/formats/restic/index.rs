use std::path::Path;

use serde::Deserialize;

use super::{decoder::Decoder, keys::Masterkey};
use crate::error::Result;

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct Index {
    #[serde(default)]
    pub supersedes: Vec<String>,
    pub packs: Vec<PackIndex>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PackIndex {
    pub id: String,
    pub blobs: Vec<BlobIndex>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BlobIndex {
    pub id: String,
    #[serde(rename = "type")]
    pub data_type: String,
    pub offset: usize,
    pub length: usize,
    pub uncompressed_length: Option<usize>,
}

impl Index {
    // Load a single index file
    pub fn from_file(masterkey: &Masterkey, path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::read(path)?;

        let decoder = Decoder::new(masterkey);
        let decoded = decoder.decrypt_and_decompress(&file)?;

        trace!("Index JSON: {}", String::from_utf8_lossy(&decoded));
        let index = serde_json::from_slice(&decoded)?;

        Ok(index)
    }

    // Load all the index files and merge them together
    pub fn from_folder(masterkey: &Masterkey, path: impl AsRef<Path>) -> Result<Self> {
        let mut indexes = Vec::new();
        for index in std::fs::read_dir(path)? {
            let index = index?;

            let index = Index::from_file(masterkey, index.path())?;

            indexes.push(index);
        }

        // merge everything into a single index
        let mut index = Index::default();

        for idx in indexes {
            index.supersedes.extend(idx.supersedes);
            index.packs.extend(idx.packs);
        }

        Ok(index)
    }

    // Search the index for the pack that contains the blob with the given ID
    pub fn find_pack(&self, id: &str) -> Option<(&PackIndex, &BlobIndex)> {
        for pack in &self.packs {
            for blob in &pack.blobs {
                if blob.id == id {
                    return Some((pack, blob));
                }
            }
        }
        None
    }
}
