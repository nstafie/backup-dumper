//! Decode a block of data that is either encrypted, compressed, or both

use aes256ctr_poly1305aes::{
    aead::{Aead, NewAead},
    Aes256CtrPoly1305Aes, Key, Nonce,
};

use super::keys::Masterkey;
use crate::error::Result;

pub struct Decoder(Box<Aes256CtrPoly1305Aes>);

impl Decoder {
    pub fn new(key: &Masterkey) -> Self {
        // reassemble the key
        let mut fullkey = Vec::new();
        fullkey.extend_from_slice(&key.encrypt);
        fullkey.extend_from_slice(&key.mac.k);
        fullkey.extend_from_slice(&key.mac.r);

        // set up the cipher
        let key = Key::from_slice(&fullkey);
        let cipher = Aes256CtrPoly1305Aes::new(key);

        Self(Box::new(cipher))
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // split the data into IV | ciphertext + MAC
        let (iv, data) = data.split_at(16);
        let nonce = Nonce::from_slice(iv);

        let plaintext = self.0.decrypt(nonce, data).unwrap();

        Ok(plaintext)
    }

    pub fn decrypt_and_decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decrypted = self.decrypt(data)?;
        let decompressed = zstd::bulk::decompress(&decrypted[1..], decrypted.len() * 20).unwrap();

        Ok(decompressed)
    }

    pub fn decrypt_and_decompress_packed(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decrypted = self.decrypt(data)?;
        let decompressed = zstd::bulk::decompress(&decrypted, decrypted.len() * 20).unwrap();

        Ok(decompressed)
    }
}
