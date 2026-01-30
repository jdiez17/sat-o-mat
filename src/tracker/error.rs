use thiserror::Error;

use crate::predict::PredictError;

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
    #[error("predict error: {0}")]
    Predict(#[from] PredictError),
}

impl From<sgp4::Error> for TrackerError {
    fn from(err: sgp4::Error) -> Self {
        TrackerError::Predict(PredictError::Propagation(err.to_string()))
    }
}
