use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid password, no matching keyfile found")]
    InvalidPassword,
    #[error("Invalid blob type: {0}")]
    InvalidBlobType(u8),
}
