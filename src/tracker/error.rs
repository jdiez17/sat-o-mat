use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("tracker already running")]
    AlreadyRunning,
    #[error("invalid tle format")]
    InvalidTleFormat,
    #[error("invalid tle: {0}")]
    InvalidTle(#[from] sgp4::TleError),
    #[error("elements error: {0}")]
    Elements(#[from] sgp4::ElementsError),
    #[error("propagation error: {0}")]
    Propagation(String),
}

impl From<sgp4::Error> for TrackerError {
    fn from(err: sgp4::Error) -> Self {
        TrackerError::Propagation(err.to_string())
    }
}
