//! Decode a block of data that is either encrypted, compressed, or both

use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, KeyInit},
    Aes256Gcm, Nonce,
};

use super::error::Error;
use crate::error::Result;

pub enum Decoder {
    Unencrypted,
    Encrypted(Box<Aes256Gcm>),
}

impl Decoder {
    pub fn new(key: impl Into<Option<[u8; 32]>>) -> Self {
        let key: Option<[u8; 32]> = key.into();

        if let Some(key) = key {
            let keybuf = GenericArray::from_slice(&key);
            let aes = Aes256Gcm::new(keybuf);
            Decoder::Encrypted(Box::new(aes))
        } else {
            Decoder::Unencrypted
        }
    }

    pub fn decode(&self, data: &[u8]) -> Result<Vec<u8>> {
        // bind data for later use
        let mut data = data;
        // create a placeholder vector so it lives long enough to keep the reference to it alive
        // for the entirety of the function - it won't allocate until something is pushed to it
        // so it's cheap enough to warrant this trick
        let mut buffer = Vec::new();

        // encrypted
        if &data[0..9] == b"duplicacy" {
            let version = data[9];
            if version != 0 {
                return Err(Error::NonzeroHeaderVersion)?;
            }

            // do we have a valid decoder with key?
            let Self::Encrypted(cipher) = self else {
                // this should only happen if there's a coding error or something weird, since we
                // check for encrypted repositories and set the key when first reading the config
                panic!("Attempting to decode encrypted file with unencrypted decoder")
            };

            // 96-bit nonce
            let nonce = &data[10..22];

            // ciphertext
            let ciphertext = &data[22..];

            // decrypt the ciphertext
            let nonce = Nonce::from_slice(nonce);
            buffer = cipher.decrypt(nonce, ciphertext)?;

            // remove the padding - this is pkcs7-like but seemingly added by duplicacy?
            // AES-GCM doesn't require padding since it uses AES-CTR internally
            // I suppose this is to keep chunk lengths somewhat obfuscated
            let mut padding_size = buffer[buffer.len() - 1] as usize;
            // see https://github.com/gilbertchen/duplicacy/blob/3a81c1065add9ec885c6fe88126446308b563e5f/src/duplicacy_chunk.go#L635C9-L635C9
            if padding_size == 0 {
                padding_size = 256;
            }
            buffer.truncate(buffer.len() - padding_size);

            data = &buffer;
        }

        // compressed
        if &data[0..4] == b"LZ4 " {
            buffer = lz4_flex::decompress_size_prepended(&data[4..])?;
        }

        // check if the buffer is empty, that means we haven't done any operations with it so
        // the data was neither encrypted nor compressed. panic instead of returning an empty buffer.
        // this should only happen due to coding errors
        if buffer.is_empty() {
            panic!("Decode failed, buffer is empty. Likely attempting to decode invalid data.");
        }

        Ok(buffer)
    }
}
