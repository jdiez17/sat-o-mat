use thiserror::Error;

#[derive(Debug, Error)]
pub enum PredictError {
    #[error("TLE directory not found: {0}")]
    DirectoryNotFound(String),
    #[error("TLE file read error: {0}")]
    FileRead(#[from] std::io::Error),
    #[error("Invalid TLE format in {file}: {message}")]
    InvalidTle { file: String, message: String },
    #[error("Propagation error: {0}")]
    Propagation(String),
    #[error("No satellites loaded")]
    #[allow(dead_code)]
    NoSatellites,
}
