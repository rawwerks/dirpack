use std::result;

#[derive(Debug, thiserror::Error)]
pub enum DirpackError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = result::Result<T, DirpackError>;
