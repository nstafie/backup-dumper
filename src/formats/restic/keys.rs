use std::path::Path;

use aes256ctr_poly1305aes::{
    aead::{Aead, NewAead},
    Aes256CtrPoly1305Aes, Key as AesKey, Nonce,
};
use serde::Deserialize;

use super::error::Error;
use crate::{error::Result, utils::from_b64};

#[derive(Deserialize, Debug)]
pub struct Key {
    pub created: String,
    pub username: String,
    pub hostname: String,
    pub kdf: String,
    #[serde(rename = "N")]
    pub n: u32,
    pub r: u32,
    pub p: u32,
    #[serde(deserialize_with = "from_b64")]
    pub salt: Vec<u8>,
    #[serde(deserialize_with = "from_b64")]
    pub data: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub struct Masterkey {
    pub mac: Mac,
    #[serde(deserialize_with = "from_b64")]
    pub encrypt: Vec<u8>,
}

#[derive(Deserialize, Debug)]
pub struct Mac {
    #[serde(deserialize_with = "from_b64")]
    pub k: Vec<u8>,
    #[serde(deserialize_with = "from_b64")]
    pub r: Vec<u8>,
}

impl Key {
    pub fn from_folder(path: impl AsRef<Path>, password: String) -> Result<Masterkey> {
        for key in std::fs::read_dir(path)? {
            let key = key?;
            let key_path = key.path();

            if !key_path.is_file() {
                continue;
            }

            // we found a working key
            if let Ok(k) = Key::from_file(key_path, &password) {
                return Ok(k);
            }
        }

        // no working key found, assume it's because of an invalid password
        Err(Error::InvalidPassword)?
    }

    pub fn from_file(path: impl AsRef<Path>, password: impl AsRef<[u8]>) -> Result<Masterkey> {
        let file = std::fs::read(path.as_ref())?;

        let json: Key = serde_json::from_slice(&file)?;

        // derive the key from the password
        let log_n = json.n.ilog2() as u8;
        let params = scrypt::Params::new(log_n, json.r, json.p, 64).unwrap();
        let mut keybuf = vec![0u8; 64];
        scrypt::scrypt(password.as_ref(), &json.salt, &params, &mut keybuf)?;

        // split the data into IV | ciphertext + MAC
        let (nonce, data) = json.data.split_at(16);

        // set up the cipher
        let key = AesKey::from_slice(&keybuf);
        let cipher = Aes256CtrPoly1305Aes::new(key);
        let nonce = Nonce::from_slice(nonce);

        // now we can decrypt the data
        let plaintext = cipher
            .decrypt(nonce, data)
            .map_err(|_| Error::InvalidPassword)?;

        let masterkey: Masterkey = serde_json::from_slice(&plaintext)?;

        Ok(masterkey)
    }
}
