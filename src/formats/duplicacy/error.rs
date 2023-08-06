use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Password required")]
    PasswordRequired,
    #[error("Invalid config header version: {0}")]
    InvalidHeaderVersion(u8),
    #[error("Attempting to decode header with version != 0")]
    NonzeroHeaderVersion,
    #[error("Mismatched hash")]
    MismatchedHash,
}
