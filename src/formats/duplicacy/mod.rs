use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use blake2::{digest::consts::U32, Blake2b, Digest};

use crate::error::Result;
use config::Config;
use data::Data;
use entry::Entry;
use index::Index;
use revision::Revision;

mod config;
mod data;
mod decoder;
mod entry;
mod index;
mod revision;

pub mod error;

type Blake2b256 = Blake2b<U32>;

#[derive(Default, Debug)]
pub struct Duplicacy {
    pub path: PathBuf,
    pub config: Config,

    snapshots: HashMap<String, Vec<Revision>>,
}

impl Duplicacy {
    pub fn from_folder(
        path: impl Into<PathBuf>,
        password: impl Into<Option<String>>,
    ) -> Result<Self> {
        let path = path.into();

        // load config
        let config = Config::from_file(path.join("config"), password.into())?;

        Ok(Duplicacy {
            path,
            config,
            ..Default::default()
        })
    }

    /// Load all the snapshots
    pub fn load_all_snapshots(&mut self) -> Result<()> {
        let mut snapshots: HashMap<String, Vec<Revision>> = HashMap::new();
        // iterate through the snapshot IDs
        for snapshot_id in std::fs::read_dir(self.path.join("snapshots"))? {
            let snapshot_id = snapshot_id?;

            // iterate through the revisions
            let mut revisions = Vec::new();
            for rev in std::fs::read_dir(snapshot_id.path())? {
                let rev = rev?;

                let revision = Revision::from_file(&self.config, rev.path())?;

                revisions.push(revision);
            }

            snapshots.insert(snapshot_id.file_name().to_string_lossy().into(), revisions);
        }

        self.snapshots = snapshots;
        Ok(())
    }

    /// Load everything
    pub fn load_all(&mut self) -> Result<()> {
        self.load_all_snapshots()?;
        Ok(())
    }

    /// Extract and recreate the files present in the latest revisions of each snapshot ID.
    ///
    /// Currently it will only attempt to restore file contents and verify it against the hash,
    /// extended metadata and permissions will be ignored.
    pub fn dump_all_files(&self, output_dir: impl AsRef<Path>) -> Result<()> {
        // iterate through all the snapshot IDs and create a subfolder for each
        for (snapshot, revisions) in self.snapshots.iter() {
            // if revisions is empty, skip this snapshot
            if revisions.is_empty() {
                continue;
            }

            // create a subfolder
            std::fs::create_dir_all(output_dir.as_ref().join(snapshot))?;

            // unwrap is safe, we've already checked for empty revisions
            let latest = revisions.iter().max_by_key(|x| x.revision).unwrap();

            let mut file_chunks = Vec::new();
            let mut index_chunks = Vec::new();
            let mut data_chunks = Vec::new();

            // read file chunks
            for hash in &latest.files {
                let path = self.config.resolve_path_from_hash(&self.path, hash)?;
                let files = Entry::from_file(&self.config, &path, hash)?;
                file_chunks.extend(files);
            }

            for hash in &latest.chunks {
                let path = self.config.resolve_path_from_hash(&self.path, hash)?;
                trace!("Attempting to read index chunk: {path:?}");
                let chunk = Index::from_file(&self.config, &path, hash)?;
                index_chunks.push(chunk);
            }

            for index in &index_chunks {
                for hash in &index.hashes {
                    let path = self.config.resolve_path_from_hash(&self.path, hash)?;
                    let data = Data::from_file(&self.config, &path, hash)?;
                    data_chunks.push(data);
                }
            }

            //debug!("File chunks: {:?}", file_chunks);
            //debug!("Index chunks: {:?}", index_chunks);
            //debug!("Data chunks: {:?}", data_chunks);

            for file in file_chunks {
                debug!("Dumping {:?}", file.path);

                // build up data - not the most efficient way but it'll do
                let mut data = Vec::new();
                for chunk in file.start_chunk..=file.end_chunk {
                    let buf = if file.start_chunk == file.end_chunk {
                        // read it all in one go
                        &data_chunks[chunk as usize].data
                            [file.start_offset as usize..file.end_offset as usize]
                    } else if chunk == file.start_chunk {
                        &data_chunks[chunk as usize].data[file.start_offset as usize..]
                    } else if chunk == file.end_chunk {
                        &data_chunks[chunk as usize].data[..file.end_offset as usize]
                    } else {
                        &data_chunks[chunk as usize].data
                    };

                    data.extend(buf);
                }

                // check the hash
                let mut hasher = Blake2b256::new();
                hasher.update(&data);
                let hash = hasher.finalize();
                if hash.as_slice() == file.hash {
                    std::fs::write(output_dir.as_ref().join(snapshot).join(file.path), data)?;
                } else {
                    return Err(error::Error::MismatchedHash)?;
                }
            }
        }

        Ok(())
    }
}
