use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::error::Result;
use config::Config;
use index::Index;
use raw_chunk::RawChunk;
use snapshot::Snapshot;

use self::snapshot::ArchiveType;

mod config;
mod decoder;
mod index;
mod raw_chunk;
mod snapshot;

#[derive(Debug)]
pub struct Knoxite {
    pub path: PathBuf,
    pub config: Config,
    pub index: Index,
    pub password: String,
    snapshots: HashMap<String, Snapshot>,
}

impl Knoxite {
    pub fn from_folder(path: impl Into<PathBuf>, password: impl Into<String>) -> Result<Self> {
        let path = path.into();
        let password = password.into();

        let config = Config::from_file(path.join("repository.knoxite"), &password)?;

        // chunk index
        let index = Index::from_file(&config, path.join("chunks").join("index"))?;

        Ok(Self {
            path,
            config,
            index,
            password,
            snapshots: HashMap::new(),
        })
    }

    pub fn load_latest_snapshots(&mut self) -> Result<()> {
        // read the latest snapshot for each volume listed in the config
        for volume in &self.config.volumes {
            let latest = volume.snapshots.last();

            if let Some(snapshot_id) = latest {
                let snapshot = Snapshot::from_file(
                    &self.config,
                    &self.path.join("snapshots").join(snapshot_id),
                )?;
                self.snapshots.insert(volume.name.to_string(), snapshot);
            }
        }

        Ok(())
    }

    pub fn load_all(&mut self) -> Result<()> {
        self.load_latest_snapshots()?;
        Ok(())
    }

    // Dump all files and folders referenced by the latest snapshots of each volume
    pub fn dump_all_files(&self, output_dir: impl AsRef<Path>) -> Result<()> {
        for snapshot in self.snapshots.values() {
            for archive in snapshot.archives.values() {
                let mut archive_path = PathBuf::from(&archive.path);
                archive_path = archive_path.strip_prefix("/").unwrap().to_path_buf();

                match archive.archive_type {
                    ArchiveType::Directory => {
                        std::fs::create_dir_all(output_dir.as_ref().join(&archive_path))?;
                    }
                    ArchiveType::File => {
                        if let Some(chunks) = &archive.chunks {
                            let mut contents = Vec::new();
                            for chunk in chunks {
                                let raw = RawChunk::from_file(
                                    &self.config,
                                    self.resolve_path(&chunk.hash),
                                )?;
                                contents.extend(raw.0);
                            }

                            // write files
                            std::fs::write(output_dir.as_ref().join(&archive_path), &contents)?;
                        }
                    }
                    ArchiveType::Symlink => {
                        // ignore
                    }
                }
            }
        }

        Ok(())
    }

    pub fn resolve_path(&self, hash: &str) -> PathBuf {
        let mut path = self
            .path
            .join("chunks")
            .join(&hash[0..2])
            .join(&hash[2..4])
            .join(hash);
        path.set_extension("0_1");

        path
    }
}
