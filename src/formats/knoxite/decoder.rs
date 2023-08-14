use std::io::Read;

use aes::cipher::{AsyncStreamCipher, KeyIvInit};
use sha2::{Digest, Sha256};

use crate::error::Result;

type Aes256CfbDec = cfb_mode::Decryptor<aes::Aes256>;

pub struct Decoder(Box<Aes256CfbDec>);

impl Decoder {
    pub fn new_password(password: impl AsRef<str>) -> Self {
        // calculate the key - this is NOT secure
        // it should be using a kdf like scrypt or argon at a minimum...
        // although the keyspace is huge, simple sha256 is FAST to calculate thanks to ASICs,
        // that makes simple passwords very easy to guess.
        let mut hasher = Sha256::new();
        hasher.update(password.as_ref());
        let hash = hasher.finalize();

        // set up the cipher
        let key = &hash;
        let iv = &hash[..16];

        let cipher = Aes256CfbDec::new(key, iv.into());

        Self(Box::new(cipher))
    }

    // this key is found in config.key -- the key looks base64 encoded, but it is read AS IS
    // that means that we don't decode the b64, we just hash the string directly
    // i'm fairly sure this isn't ideal for security either, given the reduced keyspace,
    // though it goes from <large number> to <still pretty large number>, so idk
    pub fn new_key(key: impl AsRef<str>) -> Self {
        // calculate the key
        let mut hasher = Sha256::new();
        hasher.update(key.as_ref());
        let hash = hasher.finalize();

        // set up the cipher
        let key = &hash;
        let iv = &hash[..16];

        let cipher = Aes256CfbDec::new(key, iv.into());

        Self(Box::new(cipher))
    }

    pub fn decrypt(self, data: &[u8]) -> Result<Vec<u8>> {
        //let mut plaintext = vec![0u8; data.len()];
        let mut buffer = data.to_owned();
        self.0.decrypt(&mut buffer);

        Ok(buffer)
    }

    pub fn decrypt_and_decompress(self, data: &[u8]) -> Result<Vec<u8>> {
        let decrypted = self.decrypt(data)?;

        let mut decompress = xz2::read::XzDecoder::new(&decrypted[..]);
        let mut buf = Vec::new();
        decompress.read_to_end(&mut buf)?;

        Ok(buf)
    }
}
