#![allow(dead_code)]

use std::{
    collections::HashMap,
    io::{Cursor, Read},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{error::Error, index::BlobIndex, keys::Masterkey};
use crate::{error::Result, formats::restic::decoder::Decoder};

#[derive(Debug)]
pub enum Blob {
    Data(Vec<u8>),
    Tree(Tree),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tree {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub mode: u32,
    pub mtime: String,
    pub atime: String,
    pub ctime: String,
    pub uid: u32,
    pub gid: u32,
    pub user: String,
    pub group: String,
    pub inode: u64,
    pub device_id: u64,
    pub size: Option<u64>,
    pub extended_attributes: Option<Vec<HashMap<String, String>>>,
    pub content: Option<Vec<String>>,
    pub subtree: Option<String>,
    pub links: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum NodeType {
    Dir,
    File,
}

#[derive(Debug)]
enum BlobType {
    Data {
        encrypted_length: usize,
        plaintext_hash: [u8; 32],
    },
    Tree {
        encrypted_length: usize,
        plaintext_hash: [u8; 32],
    },
    CompressedData {
        encrypted_length: usize,
        plaintext_length: usize,
        plaintext_hash: [u8; 32],
    },
    CompressedTree {
        encrypted_length: usize,
        plaintext_length: usize,
        plaintext_hash: [u8; 32],
    },
}

impl Blob {
    pub fn from_file_blobindex(
        masterkey: &Masterkey,
        file: impl AsRef<Path>,
        index: &BlobIndex,
    ) -> Result<Self> {
        let file = std::fs::read(file)?;

        // reference the blob we're interested in
        let blob_bytes = &file[index.offset..index.offset + index.length];

        // make sure our blob is the correct length
        assert_eq!(blob_bytes.len(), index.length);

        let blob = if let Some(uncompressed_len) = index.uncompressed_length {
            // blob is compressed - decrypt & decompress
            let decoder = Decoder::new(masterkey);
            let blob = decoder.decrypt_and_decompress_packed(blob_bytes)?;

            // verify
            assert_eq!(blob.len(), uncompressed_len);
            let mut hasher = Sha256::new();
            hasher.update(&blob);
            let hash = hasher.finalize();
            assert_eq!(hex::encode(hash), index.id);

            blob
        } else {
            // blob is not compressed - decrypt only
            let decoder = Decoder::new(masterkey);
            let blob = decoder.decrypt(blob_bytes)?;

            // verify
            let mut hasher = Sha256::new();
            hasher.update(&blob);
            let hash = hasher.finalize();
            assert_eq!(hex::encode(hash), index.id);

            blob
        };

        return match index.data_type.as_str() {
            "data" => Ok(Blob::Data(blob)),
            "tree" => {
                trace!("Blob tree JSON: {}", String::from_utf8_lossy(&blob));
                let tree: Tree = serde_json::from_slice(&blob)?;
                Ok(Blob::Tree(tree))
            }
            t => {
                panic!("Unsupported index type: {t}")
            }
        };
    }

    /// Read an entire pack file using the header.
    /// Pack structure: `EncryptedBlob1 || ... || EncryptedBlobN || EncryptedHeader || Header_Length`
    pub fn from_file_header(
        masterkey: &Masterkey,
        file: impl AsRef<Path>,
    ) -> Result<HashMap<String, Self>> {
        let file = std::fs::read(file)?;

        // read the header length (last 4 bytes) as u32 little-endian
        let header_length = u32::from_le_bytes(file[file.len() - 4..].try_into().unwrap());

        // read the header ... this is awkward - i like the idea, but it's not very ergonomic
        let header = &file[file.len() - (header_length + 4) as usize..file.len() - 4_usize];

        // decrypt it too
        let decoder = Decoder::new(masterkey);
        let decrypted_header = decoder.decrypt(header)?;
        let decrypted_len = decrypted_header.len();

        // parse the header
        let mut blobs: Vec<BlobType> = Vec::new();
        let mut header = Cursor::new(decrypted_header);

        while header.position() < decrypted_len as u64 {
            let blob_type = header.read_u8().expect("Read too far");
            match blob_type {
                0b00 => {
                    // data blob
                    let encrypted_length = header.read_u32::<LittleEndian>()? as usize;
                    let plaintext_hash = {
                        let mut hash = [0; 32];
                        header.read_exact(&mut hash)?;
                        hash
                    };
                    blobs.push(BlobType::Data {
                        encrypted_length,
                        plaintext_hash,
                    });
                }
                0b01 => {
                    // tree blob
                    let encrypted_length = header.read_u32::<LittleEndian>()? as usize;
                    let plaintext_hash = {
                        let mut hash = [0; 32];
                        header.read_exact(&mut hash)?;
                        hash
                    };
                    blobs.push(BlobType::Tree {
                        encrypted_length,
                        plaintext_hash,
                    });
                }
                0b10 => {
                    // compressed data blob
                    let encrypted_length = header.read_u32::<LittleEndian>()? as usize;
                    let plaintext_length = header.read_u32::<LittleEndian>()? as usize;
                    let plaintext_hash = {
                        let mut hash = [0; 32];
                        header.read_exact(&mut hash)?;
                        hash
                    };
                    blobs.push(BlobType::CompressedData {
                        encrypted_length,
                        plaintext_length,
                        plaintext_hash,
                    });
                }
                0b11 => {
                    // compressed tree blob
                    let encrypted_length = header.read_u32::<LittleEndian>()? as usize;
                    let plaintext_length = header.read_u32::<LittleEndian>()? as usize;
                    let plaintext_hash = {
                        let mut hash = [0; 32];
                        header.read_exact(&mut hash)?;
                        hash
                    };
                    blobs.push(BlobType::CompressedTree {
                        encrypted_length,
                        plaintext_length,
                        plaintext_hash,
                    });
                }
                _ => {
                    return Err(Error::InvalidBlobType(blob_type))?;
                }
            }
        }

        trace!("Parsed pack header: {:?}", blobs);

        // now we can actually parse out the data
        let mut blob_map: HashMap<String, Blob> = HashMap::new();

        let mut cursor = Cursor::new(file);

        for blob in blobs {
            match blob {
                BlobType::Data {
                    encrypted_length,
                    plaintext_hash,
                } => {
                    // read the blob
                    let mut blob = vec![0; encrypted_length];
                    cursor.read_exact(&mut blob)?;

                    // decrypt
                    let blob = decoder.decrypt(&blob)?;

                    // verify
                    let mut hasher = Sha256::new();
                    hasher.update(&blob);
                    let hash = hasher.finalize();
                    assert_eq!(hash.as_ref(), plaintext_hash);

                    // store
                    blob_map.insert(hex::encode(plaintext_hash), Blob::Data(blob));
                }
                BlobType::Tree {
                    encrypted_length,
                    plaintext_hash,
                } => {
                    // read
                    let mut blob = vec![0; encrypted_length];
                    cursor.read_exact(&mut blob)?;

                    // decrypt
                    let blob = decoder.decrypt(&blob)?;

                    // verify
                    let mut hasher = Sha256::new();
                    hasher.update(&blob);
                    let hash = hasher.finalize();
                    assert_eq!(hash.as_ref(), plaintext_hash);

                    // deserialize
                    trace!("Blob tree JSON: {}", String::from_utf8_lossy(&blob));
                    let tree: Tree = serde_json::from_slice(&blob)?;

                    // store
                    blob_map.insert(hex::encode(plaintext_hash), Blob::Tree(tree));
                }
                BlobType::CompressedData {
                    encrypted_length,
                    plaintext_length,
                    plaintext_hash,
                } => {
                    // read
                    let mut blob = vec![0; encrypted_length];
                    cursor.read_exact(&mut blob)?;

                    // decrypt
                    let blob = decoder.decrypt_and_decompress_packed(&blob)?;

                    // verify
                    assert_eq!(blob.len(), plaintext_length);
                    let mut hasher = Sha256::new();
                    hasher.update(&blob);
                    let hash = hasher.finalize();
                    assert_eq!(hash.as_ref(), plaintext_hash);

                    // store
                    blob_map.insert(hex::encode(plaintext_hash), Blob::Data(blob));
                }
                BlobType::CompressedTree {
                    encrypted_length,
                    plaintext_length,
                    plaintext_hash,
                } => {
                    // read
                    let mut blob = vec![0; encrypted_length];
                    cursor.read_exact(&mut blob)?;

                    // decrypt
                    let blob = decoder.decrypt_and_decompress_packed(&blob)?;

                    // verify
                    assert_eq!(blob.len(), plaintext_length);
                    let mut hasher = Sha256::new();
                    hasher.update(&blob);
                    let hash = hasher.finalize();
                    assert_eq!(hash.as_ref(), plaintext_hash);

                    // deserialize
                    trace!("Blob tree JSON: {}", String::from_utf8_lossy(&blob));
                    let tree: Tree = serde_json::from_slice(&blob)?;

                    // store
                    blob_map.insert(hex::encode(plaintext_hash), Blob::Tree(tree));
                }
            }
        }

        Ok(blob_map)
    }
}
