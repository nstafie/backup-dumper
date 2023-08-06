use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use blake2::{
    digest::{consts::U32, Mac},
    Blake2bMac,
};
use byteorder::{LittleEndian, ReadBytesExt};
use pbkdf2::pbkdf2_hmac;
use serde::Deserialize;
use sha2::Sha256;

use super::{decoder::Decoder, error::Error};
use crate::{error::Result, utils::from_hex};

type Blake2b256Keyed = Blake2bMac<U32>;

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub compression_level: i32,
    pub average_chunk_size: i32,
    pub max_chunk_size: i32,
    pub min_chunk_size: i32,

    #[serde(deserialize_with = "from_hex")]
    pub chunk_seed: Vec<u8>,

    pub fixed_nesting: bool,

    #[serde(deserialize_with = "from_hex")]
    pub hash_key: Vec<u8>,
    #[serde(deserialize_with = "from_hex")]
    pub id_key: Vec<u8>,
    #[serde(deserialize_with = "from_hex")]
    pub chunk_key: Vec<u8>,
    #[serde(deserialize_with = "from_hex")]
    pub file_key: Vec<u8>,

    #[serde(rename = "DataShards")]
    pub data_shards: i32,
    #[serde(rename = "ParityShards")]
    pub parity_shards: i32,

    pub rsa_public_key: String,

    #[serde(skip)]
    pub encrypted: bool,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>, password: Option<String>) -> Result<Self> {
        // read config into memory - it's a small file
        let file = std::fs::read(path)?;

        // check if the file is encrypted
        let encrypted = &file[0..9] == b"duplicacy";
        debug!("Encrypted: {}", encrypted);

        let plaintext = if encrypted {
            // make sure we have a password
            let password = password.ok_or(Error::PasswordRequired)?;

            // skip the `duplicacy` header, we've already checked it
            let mut cursor = Cursor::new(&file[9..]);

            // read version, if it's not 0 or 1 return an error
            let version = cursor.read_u8()?;
            let (salt, iterations) = match version {
                0 => {
                    // static salt and fixed number of iterations
                    let salt = b"duplicacy".to_vec();
                    let iterations: u32 = 16384;
                    (salt, iterations)
                }
                1 => {
                    // random salt and dynamic number of iterations
                    let mut salt = vec![0u8; 32];
                    cursor.read_exact(&mut salt)?;
                    let iterations = cursor.read_u32::<LittleEndian>()?;
                    (salt, iterations)
                }
                _ => return Err(Error::InvalidHeaderVersion(version))?,
            };

            debug!(
                "Header version: {}, salt: {}, iterations: {}",
                version,
                hex::encode(&salt),
                iterations
            );

            // derive the key from the password + salt + iterations using pbkdf2
            let mut key = [0u8; 32];
            pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, iterations, &mut key);

            // to keep the code simple, we'll build a version 0 block ourselves and
            // decrypt it, since everything else is formatted that way
            let mut buffer: Vec<u8> = Vec::new();
            buffer.extend_from_slice(b"duplicacy"); // header
            buffer.push(0); // version
            buffer.extend_from_slice(cursor.remaining_slice()); // data

            let decoder = Decoder::new(key);
            decoder.decode(&buffer)?
        } else {
            // an unencrypted config isn't compressed, it just needs parsing
            file
        };

        trace!("Config JSON: {}", String::from_utf8_lossy(&plaintext));

        // parse the json config
        let mut config: Config = serde_json::from_slice(&plaintext)?;
        config.encrypted = encrypted;

        Ok(config)
    }

    /// Derive the actual AES-GCM key using Blake2bKeyed with a key and derivation
    pub fn derive_key(&self, key: &[u8], derivation: &[u8]) -> Result<[u8; 32]> {
        let mut blake = Blake2b256Keyed::new_with_salt_and_personal(derivation, &[], &[])?;
        blake.update(key);
        let key = blake.finalize().into_bytes().into();

        Ok(key)
    }

    fn get_chunk_id_from_hash(&self, hash: impl AsRef<[u8]>) -> Result<Vec<u8>> {
        // hash the id with BLAKE2b256Keyed
        let mut hasher = Blake2b256Keyed::new_with_salt_and_personal(&self.id_key, &[], &[])?;
        hasher.update(hash.as_ref());
        let hash = hasher.finalize().into_bytes();

        Ok(hash.to_vec())
    }

    // hash -> chunk ID -> hex encode -> filename
    pub fn resolve_path_from_hash(
        &self,
        root: impl AsRef<Path>,
        hash: impl AsRef<[u8]>,
    ) -> Result<PathBuf> {
        let chunk_id = self.get_chunk_id_from_hash(hash)?;
        let encoded = hex::encode(chunk_id);
        let chunk_path = root
            .as_ref()
            .join("chunks")
            .join(&encoded[..2])
            .join(&encoded[2..]);

        Ok(chunk_path)
    }
}
