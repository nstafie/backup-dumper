use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use chrono::prelude::*;

use crate::error::Result;
use chunk::Chunk;
use keys::Keys;
use snapshot::Snapshot;

use self::snapshot::ItemType;

mod chunk;
mod decoder;
mod keys;
mod snapshot;

#[derive(Debug)]
pub struct BlobBackup {
    pub path: PathBuf,
    pub keys: Keys,
    pub password: String,

    pub snapshots: HashMap<String, Snapshot>,
}

impl BlobBackup {
    pub fn from_folder(path: impl Into<PathBuf>, password: impl Into<String>) -> Result<Self> {
        let path = path.into();
        let password = password.into();

        // load keys
        let keys = Keys::from_folder(path.join("keys"), &password)?;

        Ok(Self {
            path,
            keys,
            password,

            snapshots: HashMap::new(),
        })
    }

    pub fn load_all_snapshots(&mut self) -> Result<()> {
        for entry in std::fs::read_dir(self.path.join("snapshots"))? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let snapshot = Snapshot::from_file(&self.keys, &path)?;

                self.snapshots.insert(
                    path.file_name().unwrap().to_string_lossy().to_string(),
                    snapshot,
                );
            }
        }

        Ok(())
    }

    pub fn load_all(&mut self) -> Result<()> {
        self.load_all_snapshots()?;

        Ok(())
    }

    pub fn dump_all_files(&mut self, output_dir: impl AsRef<Path>) -> Result<()> {
        // find the latest snapshot
        let latest_name = self
            .snapshots
            .keys()
            .max_by_key(|x| {
                // parse the timestamp (e.g. 2023-08-15-18-36-19)
                Utc.datetime_from_str(x, "%Y-%m-%d-%H-%M-%S")
                    .expect("Failed to parse datetime from snapshot name")
            })
            .expect("Failed to find the latest snapshot");

        let latest = self.snapshots.get(latest_name).unwrap();

        // create the output_dir folder
        std::fs::create_dir_all(output_dir.as_ref())?;

        // read all the data chunks referenced by the snapshot
        let mut chunks = Vec::new();
        for hash in &latest.chunks {
            let chunk = Chunk::from_file(&self.keys, self.resolve_path(hash))?;
            chunks.push(chunk);
        }

        // ignore the folder structure, iterate only over the files
        for (name, item) in latest
            .snapshot
            .iter()
            .filter(|(_, item)| item.item_type == ItemType::File)
        {
            let filename = Path::new(name)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let range = item.range.unwrap();

            let mut data: Vec<u8> = Vec::new();

            // satisfy clippy - i'm not sure what to think of this
            for (index, chunk) in chunks
                .iter()
                .enumerate()
                .take(range.end_chunk + 1)
                .skip(range.start_chunk)
            {
                let buf = if range.start_chunk == range.end_chunk {
                    // read it all in one go
                    &chunk.data[range.start_offset..range.end_offset]
                } else if index == range.start_chunk {
                    &chunk.data[range.start_offset..]
                } else if index == range.end_chunk {
                    &chunk.data[..range.end_offset]
                } else {
                    &chunk.data
                };

                data.extend(buf);
            }

            std::fs::write(output_dir.as_ref().join(filename), data)?;
        }

        Ok(())
    }

    pub fn resolve_path(&self, hash: &str) -> PathBuf {
        self.path.join("chunks").join(hash)
    }
}
