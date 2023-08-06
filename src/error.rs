use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    // Crate errors
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    AesGcm(#[from] aes_gcm::Error),
    #[error(transparent)]
    Lz4Block(#[from] lz4_flex::block::DecompressError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Blake2InvalidLength(#[from] blake2::digest::InvalidLength),
    #[error(transparent)]
    Hex(#[from] hex::FromHexError),

    // Format errors
    #[error(transparent)]
    Duplicacy(#[from] crate::formats::duplicacy::error::Error),

    #[error("Unknown error")]
    _Unknown,
}
