use aes::{cipher::typenum::U16, Aes256};
use aes_gcm::{
    aead::{generic_array::GenericArray, Aead, KeyInit},
    AesGcm, Nonce,
};

use crate::error::Result;

pub type Aes256Gcm16ByteNonce = AesGcm<Aes256, U16>;

pub struct Decoder(Box<Aes256Gcm16ByteNonce>);

impl Decoder {
    pub fn new(key: impl AsRef<[u8]>) -> Self {
        let keybuf = GenericArray::from_slice(key.as_ref());
        let aes = Aes256Gcm16ByteNonce::new(keybuf);
        Decoder(Box::new(aes))
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let (iv, data) = data.split_at(16);
        let nonce = Nonce::from_slice(iv);

        let plaintext = self.0.decrypt(nonce, data).unwrap();

        Ok(plaintext)
    }

    pub fn decrypt_and_decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decrypted = self.decrypt(data)?;
        let decompressed = zstd::stream::decode_all(&decrypted[..])?;

        Ok(decompressed)
    }
}
