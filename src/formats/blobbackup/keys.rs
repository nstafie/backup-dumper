use std::path::Path;

use super::decoder::Decoder;
use crate::error::Result;

#[derive(Debug)]
pub struct Keys {
    pub key_salt: Vec<u8>,
    pub master_key: Vec<u8>,
    pub sha_key: Vec<u8>,
}

impl Keys {
    pub fn from_folder(path: impl AsRef<Path>, password: impl AsRef<str>) -> Result<Self> {
        let path = path.as_ref();

        // read keys/key-salt, this file isn't encrypted
        let key_salt = std::fs::read(path.join("key-salt"))?;
        // derive a key with scrypt
        let mut derived_key = vec![0u8; 32];
        let params = scrypt::Params::new(14, 8, 1, 32).unwrap();
        scrypt::scrypt(
            password.as_ref().as_bytes(),
            &key_salt,
            &params,
            &mut derived_key,
        )?;

        // read keys/master-key
        let master_key = std::fs::read(path.join("master-key"))?;
        // decrypt
        let decoder = Decoder::new(derived_key);
        let master_key = decoder.decrypt(&master_key)?;

        // read keys/sha-key
        let sha_key = std::fs::read(path.join("sha-key"))?;
        // decrypt
        let decoder = Decoder::new(&master_key);
        let sha_key = decoder.decrypt(&sha_key)?;

        Ok(Self {
            key_salt,
            master_key,
            sha_key,
        })
    }
}
