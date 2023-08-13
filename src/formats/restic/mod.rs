use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
};

use crate::{error::Result, formats::restic::pack::NodeType};
use config::Config;
use index::Index;
use keys::Key;
use pack::Blob;
use snapshot::Snapshot;

mod config;
mod decoder;
mod index;
mod keys;
mod pack;
mod snapshot;

pub mod error;

#[derive(Debug)]
pub struct Restic {
    pub path: PathBuf,
    pub config: Config,

    pub masterkey: keys::Masterkey,
    index: Index,

    snapshots: Vec<Snapshot>,
    blobs: HashMap<String, Blob>,
}

impl Restic {
    pub fn from_folder(path: impl Into<PathBuf>, password: impl Into<String>) -> Result<Self> {
        let path = path.into();

        let masterkey = Key::from_folder(path.join("keys"), password.into())?;
        let config = Config::from_file(&masterkey, path.join("config"))?;
        let index = Index::from_folder(&masterkey, path.join("index"))?;

        Ok(Self {
            path,
            config,
            masterkey,
            index,
            snapshots: Vec::new(),
            blobs: HashMap::new(),
        })
    }

    /// Load all the snapshots
    pub fn load_all_snapshots(&mut self) -> Result<()> {
        for snapshot in std::fs::read_dir(self.path.join("snapshots"))? {
            let snapshot = snapshot?;
            let snap = Snapshot::from_file(&self.masterkey, snapshot.path())?;
            self.snapshots.push(snap);
        }

        Ok(())
    }

    /// Load everything
    pub fn load_all(&mut self) -> Result<()> {
        self.load_all_snapshots()?;
        Ok(())
    }

    /// Extract and recreate the files present in the latest snapshot - directories are ignored
    pub fn dump_all_files(&mut self, output_dir: impl AsRef<Path>) -> Result<()> {
        // find the latest snapshot
        let latest = self
            .snapshots
            .iter()
            .max_by_key(|x| x.time)
            .expect("No snapshot found");

        // create initial output directory
        std::fs::create_dir_all(output_dir.as_ref())?;

        // start going through the tree and loading the referenced blobs
        let mut nodes: VecDeque<String> = VecDeque::new();
        nodes.push_back(latest.tree.to_owned());

        loop {
            // find the pack that contains the needed blob
            let (pack_index, blob_index) = self
                .index
                .find_pack(&nodes.pop_front().unwrap())
                .expect("Snapshot tree not found in index");

            // load the pack that contains the tree blob
            let blob = Blob::from_file_blobindex(
                &self.masterkey,
                &self.resolve_path(&pack_index.id),
                blob_index,
            )?;

            // parse the blob and add subtrees to be parsed
            if let Blob::Tree(tree) = &blob {
                for node in &tree.nodes {
                    if let Some(subtree) = &node.subtree {
                        nodes.push_back(subtree.to_owned());
                    }

                    if let Some(content) = &node.content {
                        // is there an extend method that we can use? unsure
                        for c in content {
                            nodes.push_back(c.to_owned());
                        }
                    }
                }
            }

            // cache blob into our blob map
            self.blobs.insert(blob_index.id.clone(), blob);

            if nodes.is_empty() {
                // we're done
                break;
            }
        }

        //trace!("Blobs parsed so far: {:#?}", self.blobs);

        // dump the files out and ignore the directory structure - much easier to do it this way
        for (_hash, blob) in self.blobs.iter() {
            if let Blob::Tree(tree) = blob {
                for node in &tree.nodes {
                    match node.node_type {
                        NodeType::Dir => {
                            // ignore this for now
                        }
                        NodeType::File => {
                            // create a file
                            if let Some(content) = &node.content {
                                let mut fulldata = Vec::new();
                                for hash in content {
                                    let data = self.blobs.get(hash).unwrap();
                                    if let Blob::Data(bytes) = data {
                                        fulldata.extend(bytes);
                                    }
                                }

                                std::fs::write(output_dir.as_ref().join(&node.name), fulldata)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn resolve_path(&self, chunk_id: &str) -> PathBuf {
        let path = self.path.join("data").join(&chunk_id[..2]).join(chunk_id);
        trace!("Resolving path: {path:?}");
        path
    }
}
