use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize, Clone, PartialEq, Eq, Copy)]
pub enum ServerError {
    #[error("Timeout")]
    Timeout,

    #[error("NoEncodedFrames")]
    NoEncodedFrames,

    #[error("NoAvailableEncoders")]
    NoAvailableEncoders,
}

