use std::result;

#[derive(Debug, thiserror::Error)]
pub enum DirpackError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("pack capacity reached; retry after {retry_after_secs}s")]
    PackBusy { retry_after_secs: u64 },
}

pub type Result<T> = result::Result<T, DirpackError>;

impl DirpackError {
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            DirpackError::PackBusy { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        }
    }
}
